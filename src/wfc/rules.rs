use crate::{ pattern::{ Pattern, PatternId }, wfc::direction::Direction };

pub(crate) const ALL_DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Right,
    Direction::Down,
    Direction::Left,
];

#[derive(Debug, Clone, Copy)]
pub struct PatternBit {
    pub word_index: usize,
    pub mask: u64,
}

pub struct AdjacencyRules {
    pub allowed: Vec<[Vec<PatternId>; 4]>,
    supporter_bits: Vec<[Vec<PatternBit>; 4]>,
}

impl AdjacencyRules {
    pub fn new(pattern_count: usize) -> Self {
        let allowed = (0..pattern_count)
            .map(|_| { [Vec::new(), Vec::new(), Vec::new(), Vec::new()] })
            .collect();

        let supporter_bits = (0..pattern_count)
            .map(|_| { [Vec::new(), Vec::new(), Vec::new(), Vec::new()] })
            .collect();

        Self { allowed, supporter_bits }
    }

    pub fn allow(
        &mut self,
        pattern_id: PatternId,
        direction: Direction,
        allowed_pattern_id: PatternId
    ) {
        let supporter_word_index = pattern_id / 64;
        let supporter_bit_index = pattern_id % 64;

        self.supporter_bits[allowed_pattern_id][direction.to_index()].push(PatternBit {
            word_index: supporter_word_index,
            mask: 1u64 << supporter_bit_index,
        });
        self.allowed[pattern_id][direction.to_index()].push(allowed_pattern_id);
    }

    pub fn get_supporter_bits(&self, pattern_id: PatternId, direction: Direction) -> &[PatternBit] {
        &self.supporter_bits[pattern_id][direction.to_index()]
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
}
