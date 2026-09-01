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
    pub removals_processed: usize,
    pub neighbor_checks: usize,
    pub allowed_entries_processed: usize,
    pub affected_patterns: usize,
    pub support_checks: usize,
    pub removed_patterns: usize,
}

use rand::{ Rng, RngExt };

use crate::{
    pattern::{ PatternId },
    wfc::{
        cell::Cell,
        entropy_heap::EntropyHeap,
        model::WfcModel,
        rules::ALL_DIRECTIONS,
        wave::{ PatternRemovalResult, Wave },
    },
};

pub struct WfcSolver {
    wave: Wave,
    propagation_queue: VecDeque<usize>,
    queued_cells: Vec<bool>,
    pending_removals: Vec<Vec<PatternId>>,
    removed_patterns_buffer: Vec<PatternId>,
    entropy_heap: EntropyHeap,
    entropy_dirty: Vec<bool>,
    dirty_entropy_cells: Vec<usize>,
    timings: SolverTimings,
    propagation_stats: PropagationStats,
    affected_marks: [Vec<u32>; 4],
    affected_patterns: [Vec<PatternId>; 4],
    affected_epoch: u32,
}

impl WfcSolver {
    pub fn new<R: Rng + ?Sized>(
        width: usize,
        height: usize,
        model: &WfcModel,
        rng: &mut R
    ) -> Self {
        let cell_count = width * height;
        let pattern_count = model.pattern_count();

        Self {
            wave: Wave::new(
                width,
                height,
                pattern_count,
                model.total_weight(),
                model.total_weight_log_weight()
            ),

            propagation_queue: VecDeque::with_capacity(cell_count),
            queued_cells: vec![false; cell_count],
            pending_removals: (0..cell_count).map(|_| Vec::new()).collect(),
            removed_patterns_buffer: Vec::with_capacity(pattern_count),
            entropy_heap: EntropyHeap::new(cell_count, pattern_count, model.initial_entropy(), rng),
            entropy_dirty: vec![false; cell_count],
            dirty_entropy_cells: Vec::with_capacity(cell_count),
            timings: SolverTimings::default(),
            propagation_stats: PropagationStats::default(),
            affected_marks: std::array::from_fn(|_| { vec![0u32; pattern_count] }),
            affected_patterns: std::array::from_fn(|_| { Vec::with_capacity(pattern_count) }),
            affected_epoch: 0,
        }
    }

    fn mark_entropy_dirty(&mut self, cell_index: usize) {
        if self.entropy_dirty[cell_index] {
            return;
        }

        self.entropy_dirty[cell_index] = true;
        self.dirty_entropy_cells.push(cell_index);
    }

    fn flush_entropy_updates(&mut self) {
        for index in 0..self.dirty_entropy_cells.len() {
            let cell_index = self.dirty_entropy_cells[index];
            self.entropy_dirty[cell_index] = false;
            let Some((entropy, possible_count)) = self.wave
                .get_cell_by_index(cell_index)
                .map(|cell| { (cell.entropy(), cell.possible_count()) }) else {
                continue;
            };

            self.entropy_heap.update(cell_index, entropy, possible_count);
        }

        self.dirty_entropy_cells.clear();
    }

    fn advance_affected_epoch(&mut self) {
        self.affected_epoch = self.affected_epoch.wrapping_add(1);

        if self.affected_epoch == 0 {
            for marks in &mut self.affected_marks {
                marks.fill(0);
            }

            self.affected_epoch = 1;
        }
    }

    fn enqueue_pattern_removal(&mut self, cell_index: usize, pattern_id: PatternId) {
        self.pending_removals[cell_index].push(pattern_id);

        if !self.queued_cells[cell_index] {
            self.queued_cells[cell_index] = true;
            self.propagation_queue.push_back(cell_index);
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
            choose_weighted_pattern(cell, model.pattern_weights(), rng)?
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

        if
            !self.wave.collapse_cell_to(
                cell_index,
                selected_pattern_id,
                model.pattern_weight(selected_pattern_id),
                model.pattern_weight_log_weight(selected_pattern_id)
            )
        {
            return None;
        }

        for index in 0..self.removed_patterns_buffer.len() {
            let removed_pattern_id = self.removed_patterns_buffer[index];
            self.enqueue_pattern_removal(cell_index, removed_pattern_id);
        }

        Some(selected_pattern_id)
    }

    pub fn observe<R: Rng + ?Sized>(
        &mut self,
        model: &WfcModel,
        rng: &mut R
    ) -> Option<(usize, PatternId)> {
        let cell_index = self.find_lowest_entropy_cell()?;
        let pattern_id = self.collapse_cell(cell_index, model, rng)?;
        Some((cell_index, pattern_id))
    }

    pub fn propagate(&mut self, model: &WfcModel) -> bool {
        let rules = model.get_rules();
        let pattern_weights = model.pattern_weights();
        let pattern_weight_log_weights = model.pattern_weight_log_weights();

        while let Some(current_index) = self.propagation_queue.pop_front() {
            self.queued_cells[current_index] = false;
            self.propagation_stats.queue_pops += 1;

            let Some((x, y)) = self.wave.index_to_coordinates(current_index) else {
                continue;
            };

            // here we take all remveols accumulated for this cell
            // new removals may be added to the same cell while this batch is being processed
            // they will then create another queue entry
            let removals = std::mem::take(&mut self.pending_removals[current_index]);

            if removals.is_empty() {
                continue;
            }

            self.propagation_stats.removals_processed += removals.len();

            self.advance_affected_epoch();
            let affected_epoch = self.affected_epoch;
            for patterns in &mut self.affected_patterns {
                patterns.clear();
            }

            for removed_pattern_id in removals {
                for direction in ALL_DIRECTIONS {
                    let direction_index = direction.to_index();
                    let allowed_patterns = rules.get_allowed_patterns(
                        removed_pattern_id,
                        direction
                    );

                    self.propagation_stats.allowed_entries_processed += allowed_patterns.len();
                    for &affected_pattern_id in allowed_patterns {
                        if
                            self.affected_marks[direction_index][affected_pattern_id] ==
                            affected_epoch
                        {
                            continue;
                        }

                        self.affected_marks[direction_index][affected_pattern_id] = affected_epoch;
                        self.affected_patterns[direction_index].push(affected_pattern_id);
                    }
                }
            }

            // since each direction is checked, we can use the affected masks to check if a neighbor pattern is still supported
            // by any of the remaining patterns in the current cell
            for direction in ALL_DIRECTIONS {
                let Some(neighbor_index) = self.wave.get_neighbor_index(x, y, direction) else {
                    continue;
                };

                self.propagation_stats.neighbor_checks += 1;
                let direction_index = direction.to_index();
                let affected_count = self.affected_patterns[direction_index].len();

                for affected_index in 0..affected_count {
                    let neighbor_pattern_id =
                        self.affected_patterns[direction_index][affected_index];
                    self.propagation_stats.affected_patterns += 1;

                    let neighbor_pattern_is_possible = self.wave
                        .get_cell_by_index(neighbor_index)
                        .is_some_and(|cell| { cell.is_pattern_possible(neighbor_pattern_id) });

                    if !neighbor_pattern_is_possible {
                        continue;
                    }

                    let still_supported = {
                        let Some(current_cell) = self.wave.get_cell_by_index(current_index) else {
                            continue;
                        };

                        // bypass the expensive check since BitSet already has the information we need
                        // pattern_id / 64
                        // pattern_id % 64
                        // 1u64 << bit_index
                        // so the hot path just become possible_words[supporter.word_index] & supporter.mask != 0
                        let supporters = rules.get_supporter_bits(neighbor_pattern_id, direction);
                        let possible_words = current_cell.possible_pattern_words();
                        let mut supported = false;

                        for supporter in supporters {
                            self.propagation_stats.support_checks += 1;

                            if (possible_words[supporter.word_index] & supporter.mask) != 0 {
                                supported = true;
                                break;
                            }
                        }
                        supported
                    };

                    if still_supported {
                        continue;
                    }

                    match
                        self.wave.remove_pattern_from_cell(
                            neighbor_index,
                            neighbor_pattern_id,
                            pattern_weights[neighbor_pattern_id],
                            pattern_weight_log_weights[neighbor_pattern_id]
                        )
                    {
                        PatternRemovalResult::Unchanged => {}
                        PatternRemovalResult::Contradiction => {
                            return false;
                        }
                        PatternRemovalResult::Removed(_) => { // (_) is done to avoid a warning about unused variable, might remove later if it becomes a problem
                            self.propagation_stats.removed_patterns += 1;
                            self.enqueue_pattern_removal(neighbor_index, neighbor_pattern_id);
                            self.mark_entropy_dirty(neighbor_index);
                        }
                    }
                }
            }
        }
        self.flush_entropy_updates();

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

    fn find_lowest_entropy_cell(&mut self) -> Option<usize> {
        self.entropy_heap.pop_min()
    }

    pub fn get_propagation_stats(&self) -> PropagationStats {
        self.propagation_stats
    }
}

pub fn choose_weighted_pattern<R: Rng + ?Sized>(
    cell: &Cell,
    pattern_weights: &[u32],
    rng: &mut R
) -> Option<PatternId> {
    if cell.is_contradiction() {
        return None;
    }

    let total_weight = cell.weight_sum();

    if total_weight == 0 {
        return None;
    }

    let mut random_weight = rng.random_range(0..total_weight);

    for pattern_id in cell.possible_pattern_ids() {
        let weight = pattern_weights[pattern_id] as u64;

        if random_weight < weight {
            return Some(pattern_id);
        }

        random_weight -= weight;
    }

    None
}
