use crate::pattern::PatternId;

pub struct Cell {
    pub possible_patterns: Vec<bool>,
}

impl Cell {
    pub fn new(pattern_count: usize) -> Self {
        Self {
            possible_patterns: vec![true; pattern_count],
        }
    }

    pub fn is_pattern_possible(&self, pattern_id: PatternId) -> bool {
        self.possible_patterns.get(pattern_id).copied().unwrap_or(false)
    }

    pub fn possible_count(&self) -> usize {
        self.possible_patterns
            .iter()
            .filter(|possible| **possible)
            .count()
    }

    pub fn is_collapsed(&self) -> bool {
        self.possible_count() == 1
    }

    pub fn is_contradiction(&self) -> bool {
        self.possible_count() == 0
    }

    pub fn remove_pattern(&mut self, pattern_id: PatternId) -> bool {
        let Some(possible) = self.possible_patterns.get_mut(pattern_id) else {
            return false;
        };

        if !*possible {
            return false;
        }

        *possible = false;

        true
    }

    pub fn possible_pattern_ids(&self) -> Vec<PatternId> {
        self.possible_patterns
            .iter()
            .enumerate()
            .filter_map(|(pattern_id, &possible)| {
                if possible { Some(pattern_id) } else { None }
            })
            .collect()
    }

    pub fn entropy(&self) -> f64 {
        // Shannon entropy formula: H = -sum(p * log2(p))
        let possible_count = self.possible_patterns
            .iter()
            .filter(|&&possible| possible)
            .count();

        if possible_count == 0 {
            return f64::INFINITY;
        }

        (possible_count as f64).log2()
    }

    pub fn collapse_to(&mut self, pattern_id: PatternId) -> bool {
        if !self.is_pattern_possible(pattern_id) {
            return false;
        }

        for (id, possible) in self.possible_patterns.iter_mut().enumerate() {
            *possible = id == pattern_id;
        }
        true
    }

    pub fn collapsed_pattern_id(&self) -> Option<PatternId> {
        if !self.is_collapsed() {
            return None;
        }

        self.possible_patterns.iter().position(|possible| *possible)
    }
}
