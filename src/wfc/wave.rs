use rand::{ Rng, RngExt };

use crate::{
    pattern::PatternId,
    wfc::{ cell::Cell, direction::Direction, rules::{ ALL_DIRECTIONS, AdjacencyRules } },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternRemovalResult {
    Unchanged,
    Removed(usize), 
    Contradiction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellConstraintResult {
    Unchanged,
    Changed(usize), //wtf is this usize? It represents the new possible count of patterns after applying the constraint
    Contradiction,
}

pub struct Wave {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    unresolved_count: usize,
    contradiction_count: usize,
}

impl Wave {
    pub fn new(width: usize, height: usize, pattern_count: usize) -> Self {
        let cell_count = width * height;
        let cells = (0..cell_count).map(|_| { Cell::new(pattern_count) }).collect();

        let unresolved_count = if pattern_count > 1 { cell_count } else { 0 };
        let contradiction_count = if pattern_count == 0 { cell_count } else { 0 };

        Self {
            width,
            height,
            cells,
            unresolved_count,
            contradiction_count,
        }
    }

    pub fn get_width(&self) -> usize {
        self.width
    }

    pub fn get_height(&self) -> usize {
        self.height
    }

    pub fn coordinates_to_index(&self, x: usize, y: usize) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }

        Some(y * self.width + x)
    }

    pub fn index_to_coordinates(&self, index: usize) -> Option<(usize, usize)> {
        if index >= self.cells.len() {
            return None;
        }

        let x = index % self.width;
        let y = index / self.width;

        Some((x, y))
    }

    pub fn get_cell(&self, x: usize, y: usize) -> Option<&Cell> {
        let index = self.coordinates_to_index(x, y)?;
        self.cells.get(index)
    }

    pub fn get_cell_by_index(&self, index: usize) -> Option<&Cell> {
        self.cells.get(index)
    }

    pub fn get_neighbor_index(&self, x: usize, y: usize, direction: Direction) -> Option<usize> {
        let (neighbor_x, neighbor_y) = match direction {
            Direction::Up => {
                if y == 0 {
                    return None;
                }

                (x, y - 1)
            }

            Direction::Right => {
                if x + 1 >= self.width {
                    return None;
                }

                (x + 1, y)
            }

            Direction::Down => {
                if y + 1 >= self.height {
                    return None;
                }

                (x, y + 1)
            }

            Direction::Left => {
                if x == 0 {
                    return None;
                }

                (x - 1, y)
            }
        };

        self.coordinates_to_index(neighbor_x, neighbor_y)
    }

    pub fn find_lowest_entropy_cell<R: Rng + ?Sized>(&self, rng: &mut R) -> Option<usize> {
        let mut lowest_entropy = f64::MAX;
        let mut candidates: Vec<usize> = Vec::new();

        for (index, cell) in self.cells.iter().enumerate() {
            if cell.is_collapsed() || cell.is_contradiction() {
                continue;
            }

            let entropy = cell.entropy();

            if entropy < lowest_entropy {
                lowest_entropy = entropy;

                candidates.clear();

                candidates.push(index);
            } else if entropy == lowest_entropy {
                candidates.push(index);
            }
        }

        if candidates.is_empty() {
            return None;
        }

        let random_index = rng.random_range(0..candidates.len());

        Some(candidates[random_index])
    }

    pub fn is_fully_collapsed(&self) -> bool {
        self.unresolved_count == 0 && self.contradiction_count == 0
    }

    pub fn has_contradiction(&self) -> bool {
        self.contradiction_count > 0
    }

    pub fn validate_constraints(&self, rules: &AdjacencyRules) -> bool {
        for current_index in 0..self.cells.len() {
            let Some(current_pattern_id) = self.cells[current_index].collapsed_pattern_id() else {
                continue;
            };

            let Some((x, y)) = self.index_to_coordinates(current_index) else {
                continue;
            };

            for direction in ALL_DIRECTIONS {
                let Some(neighbor_index) = self.get_neighbor_index(x, y, direction) else {
                    continue;
                };

                let Some(neighbor_pattern_id) =
                    self.cells[neighbor_index].collapsed_pattern_id() else {
                    continue;
                };

                let allowed_patterns = rules.get_allowed_patterns(current_pattern_id, direction);

                if !allowed_patterns.contains(&neighbor_pattern_id) {
                    println!(
                        "Invalid constraint: cell {} pattern {} -> {:?} -> cell {} pattern {}",
                        current_index,
                        current_pattern_id,
                        direction,
                        neighbor_index,
                        neighbor_pattern_id
                    );

                    return false;
                }
            }
        }

        true
    }

    fn update_cell_state_counts(&mut self, before_count: usize, after_count: usize) {
        let was_unresolved = before_count > 1;
        let is_unresolved = after_count > 1;

        if was_unresolved && !is_unresolved {
            self.unresolved_count -= 1;
        } else if !was_unresolved && is_unresolved {
            self.unresolved_count += 1;
        }

        let was_contradiction = before_count == 0;
        let is_contradiction = after_count == 0;

        if !was_contradiction && is_contradiction {
            self.contradiction_count += 1;
        } else if was_contradiction && !is_contradiction {
            self.contradiction_count -= 1;
        }
    }

    pub fn remove_pattern_from_cell(
        &mut self,
        cell_index: usize,
        pattern_id: PatternId
    ) -> PatternRemovalResult {
        let (before_count, after_count) = {
            let Some(cell) = self.cells.get_mut(cell_index) else {
                return PatternRemovalResult::Unchanged;
            };

            let before_count = cell.possible_count();

            if !cell.remove_pattern(pattern_id) {
                return PatternRemovalResult::Unchanged;
            }

            let after_count = cell.possible_count();

            (before_count, after_count)
        };

        self.update_cell_state_counts(before_count, after_count);

        if after_count == 0 {
            PatternRemovalResult::Contradiction
        } else {
            PatternRemovalResult::Removed(after_count)
        }
    }

    pub fn collapse_cell_to(&mut self, cell_index: usize, pattern_id: PatternId) -> bool {
        let (before_count, after_count) = {
            let Some(cell) = self.cells.get_mut(cell_index) else {
                return false;
            };

            let before_count = cell.possible_count();

            if !cell.collapse_to(pattern_id) {
                return false;
            }

            let after_count = cell.possible_count();

            (before_count, after_count)
        };

        self.update_cell_state_counts(before_count, after_count);

        true
    }

    pub fn get_unresolved_count(&self) -> usize {
        self.unresolved_count
    }

    pub fn get_contradiction_count(&self) -> usize {
        self.contradiction_count
    }

    pub fn intersect_cell_with_mask(
        &mut self,
        cell_index: usize,
        mask: &[u64]
    ) -> CellConstraintResult {
        let (before_count, after_count) = {
            let Some(cell) = self.cells.get_mut(cell_index) else {
                return CellConstraintResult::Unchanged;
            };

            let before_count = cell.possible_count();

            if before_count == 1 {
                let pattern_id = cell
                    .collapsed_pattern_id()
                    .expect("Collapsed cell must contain one pattern");

                let word_index = pattern_id / 64;
                let bit_index = pattern_id % 64;
                let is_supported = (mask[word_index] & (1u64 << bit_index)) != 0;

                if is_supported {
                    return CellConstraintResult::Unchanged;
                }

                let removed = cell.remove_pattern(pattern_id);
                debug_assert!(removed);
                (1, 0)
            } else {
                if !cell.intersect_with_mask(mask) {
                    return CellConstraintResult::Unchanged;
                }

                (before_count, cell.possible_count())
            }
        };

        self.update_cell_state_counts(before_count, after_count);

        if after_count == 0 {
            CellConstraintResult::Contradiction
        } else {
            CellConstraintResult::Changed(after_count)
        }
    }
}
