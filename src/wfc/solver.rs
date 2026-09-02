use std::{ collections::VecDeque, time::{ Duration, Instant } };
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

#[derive(Debug, Clone, Copy, Default)]
pub struct BacktrackingStats {
    pub decisions: usize,
    pub backtracks: usize,
    pub successful_backtracks: usize,
    pub rejected_patterns: usize,
    pub restored_patterns: usize,
    pub committed_decisions: usize,
    pub history_overflows: usize,
    pub peak_undo_entries: usize,
    pub peak_decision_depth: usize,
}

#[derive(Debug, Clone, Copy)]
struct Decision {
    cell_index: usize,
    chosen_pattern_id: PatternId,
    undo_checkpoint: u64,
}

#[derive(Debug, Clone, Copy)]
struct UndoEntry(u64);

impl UndoEntry {
    fn new(cell_index: usize, pattern_id: PatternId) -> Self {
        let packed = ((cell_index as u64) << 32) | (pattern_id as u64);
        Self(packed)
    }

    fn cell_index(self) -> usize {
        (self.0 >> 32) as usize
    }

    fn pattern_id(self) -> PatternId {
        (self.0 & (u32::MAX as u64)) as usize
    }
}

struct UndoLog {
    entries: VecDeque<UndoEntry>,
    base_position: u64,
}

impl UndoLog {
    fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            base_position: 0,
        }
    }

    fn checkpoint(&self) -> u64 {
        self.base_position + (self.entries.len() as u64)
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn push(&mut self, entry: UndoEntry) {
        self.entries.push_back(entry);
    }

    fn pop_back(&mut self) -> Option<UndoEntry> {
        self.entries.pop_back()
    }

    fn discard_before(&mut self, checkpoint: u64) {
        assert!(checkpoint >= self.base_position);

        assert!(checkpoint <= self.checkpoint());

        while self.base_position < checkpoint {
            self.entries.pop_front();
            self.base_position += 1;
        }
    }

    fn clear_committed(&mut self) {
        self.base_position = self.checkpoint();

        self.entries.clear();
    }
}

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
    backtracking_stats: BacktrackingStats,
    decision_stack: VecDeque<Decision>,
    undo_log: UndoLog,
}

const MAX_BACKTRACK_DEPTH: usize = 256;
const MAX_UNDO_ENTRIES: usize = 8_000_000; // ~ 8 million entries, ~64MB of memory for the undo stack

impl WfcSolver {
    pub fn new<R: Rng + ?Sized>(
        width: usize,
        height: usize,
        model: &WfcModel,
        rng: &mut R
    ) -> Self {
        let cell_count = width * height;
        let pattern_count = model.pattern_count();

        assert!(
            cell_count <= (u32::MAX as usize),
            "Backtracking undo entries require cell indices to fit in u32"
        );

        assert!(
            pattern_count <= (u32::MAX as usize),
            "Backtracking undo entries require pattern IDs to fit in u32"
        );

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
            backtracking_stats: BacktrackingStats::default(),
            decision_stack: VecDeque::with_capacity(MAX_BACKTRACK_DEPTH + 1),
            undo_log: UndoLog::new(),
        }
    }

    fn begin_decision(&mut self, cell_index: usize, chosen_pattern_id: PatternId) {
        let decision = Decision {
            cell_index,
            chosen_pattern_id,
            undo_checkpoint: self.undo_log.checkpoint(),
        };

        self.decision_stack.push_back(decision);
        self.backtracking_stats.decisions += 1;

        while self.decision_stack.len() > MAX_BACKTRACK_DEPTH {
            self.commit_oldest_decision();
        }

        self.backtracking_stats.peak_decision_depth =
            self.backtracking_stats.peak_decision_depth.max(self.decision_stack.len());
    }

    fn commit_oldest_decision(&mut self) {
        if self.decision_stack.pop_front().is_none() {
            return;
        }

        self.backtracking_stats.committed_decisions += 1;

        if let Some(oldest_remaining) = self.decision_stack.front() {
            self.undo_log.discard_before(oldest_remaining.undo_checkpoint);
        } else {
            self.undo_log.clear_committed();
        }
    }

    fn record_undo(&mut self, cell_index: usize, pattern_id: PatternId) {
        if self.decision_stack.is_empty() {
            return;
        }

        while self.undo_log.len() >= MAX_UNDO_ENTRIES && self.decision_stack.len() > 1 {
            self.commit_oldest_decision();
        }

        if self.undo_log.len() >= MAX_UNDO_ENTRIES {
            self.backtracking_stats.history_overflows += 1;
            self.backtracking_stats.committed_decisions += self.decision_stack.len();
            self.decision_stack.clear();
            self.undo_log.clear_committed();

            return;
        }

        self.undo_log.push(UndoEntry::new(cell_index, pattern_id));
        self.backtracking_stats.peak_undo_entries = self.backtracking_stats.peak_undo_entries.max(
            self.undo_log.len()
        );
    }

    fn remove_pattern_recorded(
        &mut self,
        cell_index: usize,
        pattern_id: PatternId,
        model: &WfcModel
    ) -> PatternRemovalResult {
        let result = self.wave.remove_pattern_from_cell(
            cell_index,
            pattern_id,
            model.pattern_weight(pattern_id),
            model.pattern_weight_log_weight(pattern_id)
        );

        if result != PatternRemovalResult::Unchanged {
            self.record_undo(cell_index, pattern_id);
        }

        result
    }

    fn clear_propagation_state(&mut self) {
        while let Some(cell_index) = self.propagation_queue.pop_front() {
            self.queued_cells[cell_index] = false;
            self.pending_removals[cell_index].clear();
        }

        self.removed_patterns_buffer.clear();

        for patterns in &mut self.affected_patterns {
            patterns.clear();
        }
    }

    fn clear_entropy_dirty_tracking(&mut self) {
        for index in 0..self.dirty_entropy_cells.len() {
            let cell_index = self.dirty_entropy_cells[index];
            self.entropy_dirty[cell_index] = false;
        }

        self.dirty_entropy_cells.clear();
    }

    fn rollback_to(&mut self, checkpoint: u64, model: &WfcModel) {
        self.clear_propagation_state();
        self.clear_entropy_dirty_tracking();

        while self.undo_log.checkpoint() > checkpoint {
            let entry = self.undo_log
                .pop_back()
                .expect("Undo log ended before the requested checkpoint");

            let cell_index = entry.cell_index();
            let pattern_id = entry.pattern_id();
            let restored = self.wave.restore_pattern_to_cell(
                cell_index,
                pattern_id,
                model.pattern_weight(pattern_id),
                model.pattern_weight_log_weight(pattern_id)
            );

            assert!(restored, "Backtracking tried to restore an already possible pattern");

            self.backtracking_stats.restored_patterns += 1;
            self.mark_entropy_dirty(cell_index);
        }

        self.flush_entropy_updates();
    }

    fn try_backtrack(&mut self, model: &WfcModel) -> bool {
        loop {
            let Some(decision) = self.decision_stack.pop_back() else {
                self.clear_propagation_state();
                self.clear_entropy_dirty_tracking();
                return false;
            };

            self.backtracking_stats.backtracks += 1;
            self.rollback_to(decision.undo_checkpoint, model);

            if self.decision_stack.is_empty() {
                self.undo_log.clear_committed();
            }

            let rejection = self.remove_pattern_recorded(
                decision.cell_index,
                decision.chosen_pattern_id,
                model
            );

            self.backtracking_stats.rejected_patterns += 1;

            match rejection {
                PatternRemovalResult::Unchanged => {
                    continue;
                }
                PatternRemovalResult::Contradiction => {
                    continue;
                }
                PatternRemovalResult::Removed { .. } => {
                    self.enqueue_pattern_removal(decision.cell_index, decision.chosen_pattern_id);

                    self.mark_entropy_dirty(decision.cell_index);
                }
            }

            let propagate_start = Instant::now();
            let propagation_success = self.propagate(model);
            self.timings.propagate += propagate_start.elapsed();
            self.timings.propagation_calls += 1;

            if propagation_success {
                self.backtracking_stats.successful_backtracks += 1;
                return true;
            }
        }
    }

    pub fn get_backtracking_stats(&self) -> BacktrackingStats {
        self.backtracking_stats
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

        self.begin_decision(cell_index, selected_pattern_id);
        self.removed_patterns_buffer.clear();

        {
            let cell = self.wave.get_cell_by_index(cell_index)?;
            self.removed_patterns_buffer.extend(
                cell
                    .possible_pattern_ids()
                    .filter(|&pattern_id| { pattern_id != selected_pattern_id })
            );
        }

        for index in 0..self.removed_patterns_buffer.len() {
            let removed_pattern_id = self.removed_patterns_buffer[index];
            match self.remove_pattern_recorded(cell_index, removed_pattern_id, model) {
                PatternRemovalResult::Unchanged => {}
                PatternRemovalResult::Removed { .. } => {
                    self.enqueue_pattern_removal(cell_index, removed_pattern_id);
                    self.mark_entropy_dirty(cell_index);
                }
                PatternRemovalResult::Contradiction => {
                    return None;
                }
            }
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

                    match self.remove_pattern_recorded(neighbor_index, neighbor_pattern_id, model) {
                        PatternRemovalResult::Unchanged => {}
                        PatternRemovalResult::Contradiction => {
                            self.mark_entropy_dirty(neighbor_index);
                            return false;
                        }
                        PatternRemovalResult::Removed { .. } => {
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
        self.backtracking_stats = BacktrackingStats::default();

        while !self.wave.is_fully_collapsed() {
            if self.wave.has_contradiction() {
                println!("Contradiction detected, attempting backtrack...");
                if !self.try_backtrack(model) {
                    return false;
                }
                continue;
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

            if !propagation_success && !self.try_backtrack(model) {
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
