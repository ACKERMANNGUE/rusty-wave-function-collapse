use std::collections::VecDeque;

use rand::{ Rng, RngExt };

use crate::{
    pattern::{ Pattern, PatternId },
    wfc::{ cell::Cell, model::WfcModel, rules::ALL_DIRECTIONS, wave::Wave },
};

pub struct WfcSolver {
    wave: Wave,
}

impl WfcSolver {
    pub fn new(width: usize, height: usize, model: &WfcModel) -> Self {
        Self {
            wave: Wave::new(width, height, model.pattern_count()),
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
        let cell_count = self.wave.get_width() * self.wave.get_height();

        if start_cell_index >= cell_count {
            return false;
        }

        let mut queue = VecDeque::new();
        let mut queued = vec![false; cell_count];

        queue.push_back(start_cell_index);
        queued[start_cell_index] = true;

        while let Some(current_index) = queue.pop_front() {
            queued[current_index] = false; // mrak the current cell as not queued because later it might be added again if its neighbors change
            let Some((x, y)) = self.wave.index_to_coordinates(current_index) else {
                continue;
            };

            let current_pattern_ids = {
                let Some(current_cell) = self.wave.get_cell_by_index(current_index) else {
                    continue;
                };

                current_cell.possible_pattern_ids()
            };

            for direction in ALL_DIRECTIONS {
                let Some(neighbor_index) = self.wave.get_neighbor_index(x, y, direction) else {
                    continue;
                };

                let neighbor_pattern_ids = {
                    let Some(neighbor_cell) = self.wave.get_cell_by_index(neighbor_index) else {
                        continue;
                    };

                    neighbor_cell.possible_pattern_ids()
                };

                let mut patterns_to_remove: Vec<PatternId> = Vec::new();

                for neighbor_pattern_id in neighbor_pattern_ids {
                    let mut is_supported = false;

                    for current_pattern_id in &current_pattern_ids {
                        let allowed_patterns = rules.get_allowed_patterns(
                            *current_pattern_id,
                            direction
                        );

                        if allowed_patterns.contains(&neighbor_pattern_id) {
                            is_supported = true;

                            break;
                        }
                    }

                    if !is_supported {
                        patterns_to_remove.push(neighbor_pattern_id);
                    }
                }

                if patterns_to_remove.is_empty() {
                    continue;
                }

                for pattern_id in patterns_to_remove {
                    self.wave.remove_pattern_from_cell(neighbor_index, pattern_id);
                }
                let neighbor_has_contradiction = self.wave
                    .get_cell_by_index(neighbor_index)
                    .map(|cell| { cell.is_contradiction() })
                    .unwrap_or(false);

                if neighbor_has_contradiction {
                    return false;
                }
                if !queued[neighbor_index] {
                    queue.push_back(neighbor_index);
                    queued[neighbor_index] = true;
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
    let possible_pattern_ids = cell.possible_pattern_ids();

    if possible_pattern_ids.is_empty() {
        return None;
    }

    let total_weight: u64 = possible_pattern_ids
        .iter()
        .map(|pattern_id| { patterns[*pattern_id].get_frequency() as u64 })
        .sum();

    if total_weight == 0 {
        return None;
    }

    let mut random_weight = rng.random_range(0..total_weight);

    for pattern_id in possible_pattern_ids {
        let weight = patterns[pattern_id].get_frequency() as u64;

        if random_weight < weight {
            return Some(pattern_id);
        }

        random_weight -= weight;
    }

    None
}
