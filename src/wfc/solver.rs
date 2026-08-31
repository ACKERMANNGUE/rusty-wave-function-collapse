use std::collections::VecDeque;

use rand::{Rng, RngExt};

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

    pub fn get_wave_mut(&mut self) -> &mut Wave {
        &mut self.wave
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

        let cell = self.wave.get_cell_by_index_mut(cell_index)?;

        if !cell.collapse_to(selected_pattern_id) {
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

        let mut queue = VecDeque::new();

        queue.push_back(start_cell_index);

        while let Some(current_index) = queue.pop_front() {
            let Some((x, y)) = self.wave.index_to_coordinates(current_index) else {
                continue;
            };

            /*
             * Copy the possible pattern IDs of the current cell.
             *
             * This avoids keeping an immutable borrow on the Wave
             * while modifying neighboring cells later.
             */
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

                /*
                 * Copy the possibilities of the neighbor before
                 * potentially mutating the cell.
                 */
                let neighbor_pattern_ids = {
                    let Some(neighbor_cell) = self.wave.get_cell_by_index(neighbor_index) else {
                        continue;
                    };

                    neighbor_cell.possible_pattern_ids()
                };

                let mut patterns_to_remove: Vec<PatternId> = Vec::new();

                /*
                 * A neighbor pattern remains possible if at least
                 * one pattern from the current cell supports it.
                 */
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

                /*
                 * Nothing changed for this neighbor, so there is
                 * nothing else to propagate from it.
                 */
                if patterns_to_remove.is_empty() {
                    continue;
                }

                let Some(neighbor_cell) = self.wave.get_cell_by_index_mut(neighbor_index) else {
                    continue;
                };

                for pattern_id in patterns_to_remove {
                    neighbor_cell.remove_pattern(pattern_id);
                }

                /*
                 * Zero remaining possibilities means that the
                 * current generation reached a contradiction.
                 */
                if neighbor_cell.is_contradiction() {
                    return false;
                }

                /*
                 * Since this neighbor changed, its constraints may
                 * now affect its own neighbors.
                 */
                queue.push_back(neighbor_index);
            }
        }

        true
    }

    pub fn solve<R: Rng + ?Sized>(&mut self, model: &WfcModel, rng: &mut R) -> bool {
        while !self.wave.is_fully_collapsed() {
            if self.wave.has_contradiction() {
                return false;
            }

            /*
             * Observation chooses the lowest entropy cell
             * and collapses it to one pattern.
             */
            let Some((cell_index, _pattern_id)) = self.observe(model, rng) else {
                return self.wave.is_fully_collapsed();
            };

            /*
             * Propagation removes incompatible patterns
             * throughout the Wave.
             */
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

    /*
     * Frequencies learned from the input image are used
     * as selection weights.
     */
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
