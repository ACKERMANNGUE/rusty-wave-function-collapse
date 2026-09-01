use std::{ collections::VecDeque, time::{ Duration, Instant } };

#[derive(Debug, Clone, Copy, Default)]
pub struct SolverTimings {
    pub observe: Duration,
    pub propagate: Duration,
    pub observations: usize,
    pub propagation_calls: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PropagationStats {
    pub queue_pops: usize,
    pub neighbor_checks: usize,
    pub collapsed_current: usize,
    pub collapsed_neighbor: usize,
    pub changed_neighbors: usize,
    pub patterns_iterated: usize,
    pub allowed_entries_processed: usize,
}

use rand::{ Rng, RngExt };

use crate::{
    pattern::{ Pattern, PatternId },
    wfc::{
        cell::Cell,
        entropy_buckets::EntropyBuckets,
        model::WfcModel,
        rules::ALL_DIRECTIONS,
        wave::{ CellConstraintResult, Wave },
    },
};

pub struct WfcSolver {
    wave: Wave,
    propagation_queue: VecDeque<usize>,
    queued_epoch: Vec<u32>,
    current_epoch: u32,
    supported_masks: [Vec<u64>; 4],
    entropy_buckets: EntropyBuckets,
    timings: SolverTimings,
    propagation_stats: PropagationStats,
}

impl WfcSolver {
    pub fn new(width: usize, height: usize, model: &WfcModel) -> Self {
        let cell_count = width * height;
        let word_count = model.pattern_count().div_ceil(64);

        Self {
            wave: Wave::new(width, height, model.pattern_count()),
            propagation_queue: VecDeque::with_capacity(cell_count),
            queued_epoch: vec![0; cell_count],
            current_epoch: 0,
            supported_masks: [
                vec![0u64; word_count],
                vec![0u64; word_count],
                vec![0u64; word_count],
                vec![0u64; word_count],
            ],
            entropy_buckets: EntropyBuckets::new(model.pattern_count(), cell_count),
            timings: SolverTimings::default(),
            propagation_stats: PropagationStats::default(),
        }
    }

    pub fn get_timings(&self) -> SolverTimings {
        self.timings
    }

    fn advance_epoch(&mut self) {
        self.current_epoch = self.current_epoch.wrapping_add(1);
        // reset the queued_epoch vector if it wraps around to 0
        // this is a safeguard against potential overflow, ensuring that the epoch counter remains valid
        if self.current_epoch == 0 {
            self.queued_epoch.fill(0);
            self.current_epoch = 1;
        }
    }

    pub fn get_wave(&self) -> &Wave {
        &self.wave
    }

    pub fn collapse_cell<R: Rng + ?Sized>(
        &mut self,
        cell_index: usize,
        model: &WfcModel,
        rng: &mut R
    ) -> Option<PatternId> {
        let selected_pattern_id = {
            let cell = self.wave.get_cell_by_index(cell_index)?;
            choose_weighted_pattern(cell, model.get_patterns(), rng)?
        };

        if !self.wave.collapse_cell_to(cell_index, selected_pattern_id) {
            return None;
        }

        Some(selected_pattern_id)
    }

    pub fn observe<R: Rng + ?Sized>(
        &mut self,
        model: &WfcModel,
        rng: &mut R
    ) -> Option<(usize, PatternId)> {
        let cell_index = self.find_lowest_entropy_cell(rng)?;
        let pattern_id = self.collapse_cell(cell_index, model, rng)?;
        Some((cell_index, pattern_id))
    }

    pub fn propagate(&mut self, start_cell_index: usize, model: &WfcModel) -> bool {
        let rules = model.get_rules();

        if start_cell_index >= self.queued_epoch.len() {
            return false;
        }

        self.propagation_queue.clear();
        self.advance_epoch();

        let epoch = self.current_epoch;

        self.propagation_queue.push_back(start_cell_index);
        self.queued_epoch[start_cell_index] = epoch;

        while let Some(current_index) = self.propagation_queue.pop_front() {
            self.queued_epoch[current_index] = 0;
            self.propagation_stats.queue_pops += 1;

            let Some((x, y)) = self.wave.index_to_coordinates(current_index) else {
                continue;
            };

            let collapsed_pattern_id = {
                let Some(current_cell) = self.wave.get_cell_by_index(current_index) else {
                    continue;
                };

                if current_cell.is_contradiction() {
                    return false;
                }

                current_cell.collapsed_pattern_id()
            };

            // build the supported-pattern masks only if the current cell is not collapsed
            // it's done in a single iteration to avoid redundant computations and improve efficiency
            if collapsed_pattern_id.is_none() {
                for mask in &mut self.supported_masks {
                    mask.fill(0);
                }

                let Some(current_cell) = self.wave.get_cell_by_index(current_index) else {
                    continue;
                };

                for current_pattern_id in current_cell.possible_pattern_ids() {
                    self.propagation_stats.patterns_iterated += 1;

                    for direction in ALL_DIRECTIONS {
                        let direction_index = direction.to_index();

                        let allowed_patterns = rules.get_allowed_patterns(
                            current_pattern_id,
                            direction
                        );

                        self.propagation_stats.allowed_entries_processed += allowed_patterns.len();

                        for &allowed_pattern_id in allowed_patterns {
                            let word_index = allowed_pattern_id / 64;
                            let bit_index = allowed_pattern_id % 64;
                            self.supported_masks[direction_index][word_index] |= 1u64 << bit_index;
                        }
                    }
                }
            }

            for direction in ALL_DIRECTIONS {
                let Some(neighbor_index) = self.wave.get_neighbor_index(x, y, direction) else {
                    continue;
                };

                self.propagation_stats.neighbor_checks += 1;

                if
                    self.wave
                        .get_cell_by_index(neighbor_index)
                        .is_some_and(|cell| cell.is_collapsed())
                {
                    self.propagation_stats.collapsed_neighbor += 1;
                }

                let constraint_result = if let Some(pattern_id) = collapsed_pattern_id {
                    // Fast path, because it avoids the overhead of iterating through all possible patterns and computing the allowed patterns for each direction
                    self.propagation_stats.collapsed_current += 1;
                    let allowed_mask = rules.get_allowed_mask(pattern_id, direction);
                    self.wave.intersect_cell_with_mask(neighbor_index, allowed_mask)
                } else {
                    // General path, because the current cell is not collapsed, so we need to compute the allowed patterns for each possible pattern in the current cell
                    let direction_index = direction.to_index();

                    self.wave.intersect_cell_with_mask(
                        neighbor_index,
                        &self.supported_masks[direction_index]
                    )
                };

                match constraint_result {
                    CellConstraintResult::Unchanged => {
                        continue;
                    }
                    CellConstraintResult::Contradiction => {
                        return false;
                    }
                    CellConstraintResult::Changed(possible_count) => {
                        self.propagation_stats.changed_neighbors += 1;
                        self.entropy_buckets.push(neighbor_index, possible_count);
                    }
                }

                if self.queued_epoch[neighbor_index] != epoch {
                    self.propagation_queue.push_back(neighbor_index);
                    self.queued_epoch[neighbor_index] = epoch;
                }
            }
        }

        true
    }

    pub fn solve<R: Rng + ?Sized>(&mut self, model: &WfcModel, rng: &mut R) -> bool {
        self.timings = SolverTimings::default();
        self.propagation_stats = PropagationStats::default();

        while !self.wave.is_fully_collapsed() {
            if self.wave.has_contradiction() {
                return false;
            }

            let observe_start = Instant::now();

            let observation = self.observe(model, rng);

            self.timings.observe += observe_start.elapsed();
            self.timings.observations += 1;

            let Some((cell_index, _pattern_id)) = observation else {
                return self.wave.is_fully_collapsed();
            };

            let propagate_start = Instant::now();

            let propagation_success = self.propagate(cell_index, model);

            self.timings.propagate += propagate_start.elapsed();
            self.timings.propagation_calls += 1;

            if !propagation_success {
                return false;
            }
        }

        true
    }

    fn find_lowest_entropy_cell<R: Rng + ?Sized>(&mut self, rng: &mut R) -> Option<usize> {
        let wave = &self.wave;

        self.entropy_buckets.pop_lowest(rng, |cell_index| {
            wave.get_cell_by_index(cell_index)
                .map(|cell| cell.possible_count())
                .unwrap_or(0)
        })
    }

    pub fn get_propagation_stats(&self) -> PropagationStats {
        self.propagation_stats
    }
}

pub fn choose_weighted_pattern<R: Rng + ?Sized>(
    cell: &Cell,
    patterns: &[Pattern],
    rng: &mut R
) -> Option<PatternId> {
    if cell.is_contradiction() {
        return None;
    }

    let total_weight: u64 = cell
        .possible_pattern_ids()
        .map(|pattern_id| { patterns[pattern_id].get_frequency() as u64 })
        .sum();

    if total_weight == 0 {
        return None;
    }

    let mut random_weight = rng.random_range(0..total_weight);

    for pattern_id in cell.possible_pattern_ids() {
        let weight = patterns[pattern_id].get_frequency() as u64;

        if random_weight < weight {
            return Some(pattern_id);
        }

        random_weight -= weight;
    }

    None
}
