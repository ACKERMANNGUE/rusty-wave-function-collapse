use rand::{ Rng, RngExt };

use crate::wfc::{ cell::Cell, direction::Direction, rules::{ALL_DIRECTIONS, AdjacencyRules} };

pub struct Wave {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
}

impl Wave {
    pub fn new(width: usize, height: usize, pattern_count: usize) -> Self {
        let cell_count = width * height;

        let cells = (0..cell_count).map(|_| { Cell::new(pattern_count) }).collect();

        Self {
            width,
            height,
            cells,
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

    pub fn get_cell_by_index_mut(&mut self, index: usize) -> Option<&mut Cell> {
        self.cells.get_mut(index)
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
        self.cells.iter().all(|cell| { cell.is_collapsed() })
    }

    pub fn has_contradiction(&self) -> bool {
        self.cells.iter().any(|cell| { cell.is_contradiction() })
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
}
