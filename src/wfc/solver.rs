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
    queued: Vec<bool>,
    patterns_to_remove: Vec<PatternId>,
}

impl WfcSolver {
    pub fn new(width: usize, height: usize, model: &WfcModel) -> Self {
        let cell_count = width * height;

        Self {
            wave: Wave::new(width, height, model.pattern_count()),
            propagation_queue: VecDeque::with_capacity(cell_count),
            queued: vec![false; cell_count],
            patterns_to_remove: Vec::with_capacity(model.pattern_count()),
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

        self.propagation_queue.push_back(start_cell_index);
        self.queued[start_cell_index] = true;

        while let Some(current_index) = self.propagation_queue.pop_front() {
            self.queued[current_index] = false;

            let Some((x, y)) = self.wave.index_to_coordinates(current_index) else {
                continue;
            };

            for direction in ALL_DIRECTIONS {
                let Some(neighbor_index) = self.wave.get_neighbor_index(x, y, direction) else {
                    continue;
                };

                self.patterns_to_remove.clear();

                // READ PHASE: borrow the current cell and neighbor cell immutably to check for supported patterns
                {
                    let Some(current_cell) = self.wave.get_cell_by_index(current_index) else {
                        continue;
                    };

                    let Some(neighbor_cell) = self.wave.get_cell_by_index(neighbor_index) else {
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
                            self.patterns_to_remove.push(neighbor_pattern_id);
                        }
                    }
                }

                if self.patterns_to_remove.is_empty() {
                    continue;
                }

                // WRITE PHASE: borrow the neighbor cell mutably to remove unsupported patterns
                for &pattern_id in &self.patterns_to_remove {
                    match self.wave.remove_pattern_from_cell(neighbor_index, pattern_id) {
                        PatternRemovalResult::Contradiction => {
                            return false;
                        }
                        PatternRemovalResult::Removed => {}
                        PatternRemovalResult::NotRemoved => {}
                    }
                }

                if !self.queued[neighbor_index] {
                    self.propagation_queue.push_back(neighbor_index);
                    self.queued[neighbor_index] = true;
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
