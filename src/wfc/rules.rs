use crate::{ pattern::{ Pattern, PatternId }, wfc::direction::Direction };

pub(crate) const ALL_DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Right,
    Direction::Down,
    Direction::Left,
];

pub struct AdjacencyRules {
    pub allowed: Vec<[Vec<PatternId>; 4]>,
}

impl AdjacencyRules {
    pub fn new(pattern_count: usize) -> Self {
        let allowed = (0..pattern_count)
            .map(|_| { [Vec::new(), Vec::new(), Vec::new(), Vec::new()] })
            .collect();

        Self { allowed }
    }

    pub fn allow(
        &mut self,
        pattern_id: PatternId,
        direction: Direction,
        allowed_pattern_id: PatternId
    ) {
        let dir_index = direction.to_index();
        self.allowed[pattern_id][dir_index].push(allowed_pattern_id);
    }

    pub fn get_allowed_patterns(
        &self,
        pattern_id: PatternId,
        direction: Direction
    ) -> &[PatternId] {
        let dir_index = direction.to_index();
        &self.allowed[pattern_id][dir_index]
    }

    pub fn compute_rules(&mut self, patterns: &[Pattern]) {
        for pattern_a in patterns {
            for pattern_b in patterns {
                for direction in ALL_DIRECTIONS {
                    if pattern_a.overlaps(pattern_b, direction) {
                        self.allow(pattern_a.get_id(), direction, pattern_b.get_id());
                    }
                }
            }
        }
    }

    pub fn count_rules(&self) -> usize {
        self.allowed
            .iter()
            .map(|directions| {
                directions
                    .iter()
                    .map(|allowed_patterns| allowed_patterns.len())
                    .sum::<usize>()
            })
            .sum()
    }

    pub fn validate_rules_symmetry(&self) -> bool {
        for (pattern_id, directions) in self.allowed.iter().enumerate() {
            for (dir_index, allowed_patterns) in directions.iter().enumerate() {
                let direction = ALL_DIRECTIONS[dir_index];
                let opposite_direction = direction.opposite();
                let opposite_dir_index = opposite_direction.to_index();

                for &allowed_pattern_id in allowed_patterns {
                    let opposite_allowed_patterns = &self.allowed[allowed_pattern_id][opposite_dir_index];
                    if !opposite_allowed_patterns.contains(&(pattern_id as PatternId)) {
                        return false;
                    }
                }
            }
        }
        true
    }
}
