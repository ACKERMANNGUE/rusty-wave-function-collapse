use std::collections::VecDeque;

use rand::{ Rng, RngExt };

use crate::{
    pattern::{ Pattern, PatternId },
    wfc::{
        cell::Cell,
        model::WfcModel,
        rules::ALL_DIRECTIONS,
        wave::{ PatternRemovalResult, Wave },
    },
};

pub struct WfcSolver {
    wave: Wave,
    propagation_queue: VecDeque<usize>,
    queued_epoch: Vec<u32>,
    current_epoch: u32,
    patterns_to_remove: Vec<PatternId>,
}

impl WfcSolver {
    pub fn new(width: usize, height: usize, model: &WfcModel) -> Self {
        let cell_count = width * height;

        Self {
            wave: Wave::new(width, height, model.pattern_count()),
            propagation_queue: VecDeque::with_capacity(cell_count),
            queued_epoch: vec![0; cell_count],
            current_epoch: 0,
            patterns_to_remove: Vec::with_capacity(model.pattern_count()),
        }
    }

    fn advance_epoch(&mut self) {
        self.current_epoch = self.current_epoch.wrapping_add(1);
        // reset the queued_epoch vector if it wraps around to 0
        // zhis is a safeguard against potential overflow, ensuring that the epoch counter remains valid
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
        let cell_index = self.wave.find_lowest_entropy_cell(rng)?;
        let pattern_id = self.collapse_cell(cell_index, model, rng)?;
        Some((cell_index, pattern_id))
    }

    pub fn propagate(&mut self, start_cell_index: usize, model: &WfcModel) -> bool {
        let rules = model.get_rules();

        if start_cell_index >= self.queued_epoch.len() {
            return false;
        }

        // reset the propagation queue and patterns_to_remove vector to prepare for the propagation process
        self.propagation_queue.clear();
        self.patterns_to_remove.clear();

        self.advance_epoch();

        let epoch = self.current_epoch;

        self.propagation_queue.push_back(start_cell_index);
        self.queued_epoch[start_cell_index] = epoch;

        while let Some(current_index) = self.propagation_queue.pop_front() {
            // the cell is no longer queued and may be queued again later during this propagation
            self.queued_epoch[current_index] = 0;

            let Some((x, y)) = self.wave.index_to_coordinates(current_index) else {
                continue;
            };

            for direction in ALL_DIRECTIONS {
                let Some(neighbor_index) = self.wave.get_neighbor_index(x, y, direction) else {
                    continue;
                };

                self.patterns_to_remove.clear();


                // READ PHASE: Determine which patterns to remove from the neighbor cell
                {
                    let wave = &self.wave;
                    let patterns_to_remove = &mut self.patterns_to_remove;

                    let Some(current_cell) = wave.get_cell_by_index(current_index) else {
                        continue;
                    };

                    let Some(neighbor_cell) = wave.get_cell_by_index(neighbor_index) else {
                        continue;
                    };

                    for neighbor_pattern_id in neighbor_cell.possible_pattern_ids() {
                        let is_supported = current_cell
                            .possible_pattern_ids()
                            .any(|current_pattern_id| {
                                rules
                                    .get_allowed_patterns(current_pattern_id, direction)
                                    .contains(&neighbor_pattern_id)
                            });

                        if !is_supported {
                            patterns_to_remove.push(neighbor_pattern_id);
                        }
                    }
                }

                if self.patterns_to_remove.is_empty() {
                    continue;
                }

                // WRITE PHASE: Remove the patterns from the neighbor cell and check for contradictions
                for &pattern_id in &self.patterns_to_remove {
                    match self.wave.remove_pattern_from_cell(neighbor_index, pattern_id) {
                        PatternRemovalResult::Contradiction => {
                            return false;
                        }
                        PatternRemovalResult::Removed => {}
                        PatternRemovalResult::NotRemoved => {}
                    }
                }

                // Add the neighbor cell to the queue if it hasn't been queued in this epoch
                if self.queued_epoch[neighbor_index] != epoch {
                    self.propagation_queue.push_back(neighbor_index);

                    self.queued_epoch[neighbor_index] = epoch;
                }
            }
        }

        true
    }
    
    pub fn solve<R: Rng + ?Sized>(&mut self, model: &WfcModel, rng: &mut R) -> bool {
        while !self.wave.is_fully_collapsed() {
            if self.wave.has_contradiction() {
                return false;
            }

            let Some((cell_index, _pattern_id)) = self.observe(model, rng) else {
                return self.wave.is_fully_collapsed();
            };

            if !self.propagate(cell_index, model) {
                return false;
            }
        }

        true
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
