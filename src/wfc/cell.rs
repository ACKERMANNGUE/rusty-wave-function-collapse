use crate::{ pattern::PatternId, wfc::bitset::BitSet };

pub struct Cell {
    possible_patterns: BitSet,
    possible_count: usize,
}

impl Cell {
    pub fn new(pattern_count: usize) -> Self {
        Self {
            possible_patterns: BitSet::full(pattern_count),
            possible_count: pattern_count,
        }
    }

    pub fn is_pattern_possible(&self, pattern_id: PatternId) -> bool {
        self.possible_patterns.contains(pattern_id)
    }

    pub fn is_collapsed(&self) -> bool {
        self.possible_count == 1
    }

    pub fn is_contradiction(&self) -> bool {
        self.possible_count == 0
    }

    pub fn remove_pattern(&mut self, pattern_id: PatternId) -> bool {
        if self.possible_patterns.remove(pattern_id) {
            self.possible_count -= 1;
            true
        } else {
            false
        }
    }

    pub fn possible_pattern_ids(&self) -> impl Iterator<Item = PatternId> + '_ {
        self.possible_patterns.iter_ones()
    }

    pub fn entropy(&self) -> f64 {
        self.possible_count as f64
    }

    pub fn possible_count(&self) -> usize {
        self.possible_count
    }

    pub fn collapse_to(&mut self, pattern_id: PatternId) -> bool {
        if !self.is_pattern_possible(pattern_id) {
            return false;
        }

        self.possible_patterns.keep_only(pattern_id);
        self.possible_count = 1;

        true
    }

    pub fn collapsed_pattern_id(&self) -> Option<PatternId> {
        if !self.is_collapsed() {
            return None;
        }

        self.possible_patterns.iter_ones().next()
    }

}
