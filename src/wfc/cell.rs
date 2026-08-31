use crate::pattern::PatternId;

pub struct Cell {
    possible_patterns: Vec<bool>,
    possible_count: usize,
}

impl Cell {
    pub fn new(pattern_count: usize) -> Self {
        Self {
            possible_patterns: vec![true; pattern_count],
            possible_count: pattern_count,
        }
    }

    pub fn is_pattern_possible(&self, pattern_id: PatternId) -> bool {
        self.possible_patterns.get(pattern_id).copied().unwrap_or(false)
    }

    pub fn is_collapsed(&self) -> bool {
        self.possible_count == 1
    }

    pub fn is_contradiction(&self) -> bool {
        self.possible_count == 0
    }

    pub fn remove_pattern(&mut self, pattern_id: PatternId) -> bool {
        let Some(possible) = self.possible_patterns.get_mut(pattern_id) else {
            return false;
        };

        if !*possible {
            return false;
        }

        *possible = false;

        self.possible_count -= 1;

        true
    }

    // thanks ChatGPT for helping me write this function 
    // this function returns an iterator over the possible pattern IDs for the cell instead of collect which creates a new vector
    // this is more efficient as it avoids unnecessary allocations and copying of data
    pub fn possible_pattern_ids(&self) -> impl Iterator<Item = PatternId> + '_ {
        self.possible_patterns
            .iter()
            .enumerate()
            .filter_map(|(pattern_id, &possible)| {
                if possible { Some(pattern_id) } else { None }
            })
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

        for (id, possible) in self.possible_patterns.iter_mut().enumerate() {
            *possible = id == pattern_id;
        }

        self.possible_count = 1;

        true
    }

    pub fn collapsed_pattern_id(&self) -> Option<PatternId> {
        if !self.is_collapsed() {
            return None;
        }

        self.possible_patterns.iter().position(|possible| *possible)
    }
}
