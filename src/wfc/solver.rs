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
    pub affected_patterns: usize,
    pub support_checks: usize,
    pub removed_patterns: usize,
}

use rand::{ Rng, RngExt };

use crate::{
    pattern::{ Pattern, PatternId },
    wfc::{
        cell::Cell,
        entropy_buckets::EntropyBuckets,
        model::WfcModel,
        rules::ALL_DIRECTIONS,
        wave::{ PatternRemovalResult, Wave },
    },
};

#[derive(Debug, Clone, Copy)]
struct PatternRemoval {
    cell_index: usize,
    pattern_id: PatternId,
}

pub struct WfcSolver {
    wave: Wave,
    propagation_queue: VecDeque<PatternRemoval>,
    entropy_buckets: EntropyBuckets,
    timings: SolverTimings,
    propagation_stats: PropagationStats,
    removed_patterns_buffer: Vec<PatternId>,
}

impl WfcSolver {
    pub fn new(width: usize, height: usize, model: &WfcModel) -> Self {
        let cell_count = width * height;
        let pattern_count = model.pattern_count();

        Self {
            wave: Wave::new(width, height, pattern_count),
            propagation_queue: VecDeque::with_capacity(cell_count),
            entropy_buckets: EntropyBuckets::new(pattern_count, cell_count),
            timings: SolverTimings::default(),
            propagation_stats: PropagationStats::default(),
            removed_patterns_buffer: Vec::with_capacity(pattern_count),
        }
    }

    pub fn get_timings(&self) -> SolverTimings {
        self.timings
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
        self.removed_patterns_buffer.clear();
        {
            let cell = self.wave.get_cell_by_index(cell_index)?;
            self.removed_patterns_buffer.extend(
                cell
                    .possible_pattern_ids()
                    .filter(|&pattern_id| { pattern_id != selected_pattern_id })
            );
        }

        if !self.wave.collapse_cell_to(cell_index, selected_pattern_id) {
            return None;
        }

        for &removed_pattern_id in &self.removed_patterns_buffer {
            self.propagation_queue.push_back(PatternRemoval {
                cell_index,
                pattern_id: removed_pattern_id,
            });
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

    pub fn propagate(&mut self, model: &WfcModel) -> bool {
        let rules = model.get_rules();

        while let Some(removal) = self.propagation_queue.pop_front() {
            self.propagation_stats.queue_pops += 1;

            let Some((x, y)) = self.wave.index_to_coordinates(removal.cell_index) else {
                continue;
            };

            for direction in ALL_DIRECTIONS {
                let Some(neighbor_index) = self.wave.get_neighbor_index(x, y, direction) else {
                    continue;
                };

                self.propagation_stats.neighbor_checks += 1;
                let affected_patterns = rules.get_allowed_patterns(removal.pattern_id, direction);
                self.propagation_stats.affected_patterns += affected_patterns.len();

                for &neighbor_pattern_id in affected_patterns {
                    let neighbor_pattern_is_possible = self.wave
                        .get_cell_by_index(neighbor_index)
                        .is_some_and(|cell| { cell.is_pattern_possible(neighbor_pattern_id) });

                    if !neighbor_pattern_is_possible {
                        continue;
                    }

                    let still_supported = {
                        let Some(current_cell) = self.wave.get_cell_by_index(
                            removal.cell_index
                        ) else {
                            continue;
                        };

                        let supporters = rules.get_supporters(neighbor_pattern_id, direction);
                        let mut supported = false;

                        for &supporter_pattern_id in supporters {
                            self.propagation_stats.support_checks += 1;

                            if current_cell.is_pattern_possible(supporter_pattern_id) {
                                supported = true;
                                break;
                            }
                        }

                        supported
                    };

                    if still_supported {
                        continue;
                    }

                    match self.wave.remove_pattern_from_cell(neighbor_index, neighbor_pattern_id) {
                        PatternRemovalResult::Unchanged => {}
                        PatternRemovalResult::Contradiction => {
                            return false;
                        }
                        PatternRemovalResult::Removed(possible_count) => {
                            self.propagation_stats.removed_patterns += 1;
                            self.entropy_buckets.push(neighbor_index, possible_count);
                            self.propagation_queue.push_back(PatternRemoval {
                                cell_index: neighbor_index,
                                pattern_id: neighbor_pattern_id,
                            });
                        }
                    }
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

            let Some((_cell_index, _pattern_id)) = observation else {
                return self.wave.is_fully_collapsed();
            };

            let propagate_start = Instant::now();
            let propagation_success = self.propagate(model);

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
