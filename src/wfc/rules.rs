use crate::{ pattern::{ Pattern, PatternId }, wfc::direction::Direction };

pub(crate) const ALL_DIRECTIONS: [Direction; 4] = [
    Direction::Up,
    Direction::Right,
    Direction::Down,
    Direction::Left,
];

pub struct AdjacencyRules {
    pub allowed: Vec<[Vec<PatternId>; 4]>,
    allowed_masks: Vec<[Vec<u64>; 4]>,
    supporters: Vec<[Vec<PatternId>; 4]>,
}

impl AdjacencyRules {
    pub fn new(pattern_count: usize) -> Self {
        let allowed = (0..pattern_count)
            .map(|_| { [Vec::new(), Vec::new(), Vec::new(), Vec::new()] })
            .collect();

        let word_count = pattern_count.div_ceil(64);
        let allowed_masks = (0..pattern_count)
            .map(|_| { std::array::from_fn(|_| vec![0u64; word_count]) })
            .collect();

        let supporters = (0..pattern_count)
            .map(|_| { [Vec::new(), Vec::new(), Vec::new(), Vec::new()] })
            .collect();

        Self { allowed, allowed_masks, supporters }
    }

    pub fn allow(
        &mut self,
        pattern_id: PatternId,
        direction: Direction,
        allowed_pattern_id: PatternId
    ) {
        let dir_index = direction.to_index();
        self.allowed[pattern_id][dir_index].push(allowed_pattern_id);

        let word_index = allowed_pattern_id / 64;
        let bit_index = allowed_pattern_id % 64;
        self.allowed_masks[pattern_id][dir_index][word_index] |= 1u64 << bit_index;
        self.supporters[allowed_pattern_id][dir_index].push(pattern_id);
    }

    pub fn get_supporters(&self, pattern_id: PatternId, direction: Direction) -> &[PatternId] {
        let dir_index = direction.to_index();
        &self.supporters[pattern_id][dir_index]
    }

    pub fn get_allowed_patterns(
        &self,
        pattern_id: PatternId,
        direction: Direction
    ) -> &[PatternId] {
        let dir_index = direction.to_index();
        &self.allowed[pattern_id][dir_index]
    }

    pub fn get_allowed_mask(&self, pattern_id: PatternId, direction: Direction) -> &[u64] {
        let dir_index = direction.to_index();
        &self.allowed_masks[pattern_id][dir_index]
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
