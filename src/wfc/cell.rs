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

    pub fn get_frequencies(&self) -> Vec<usize> {
        self.possible_patterns
            .iter()
            .map(|&possible| if possible { 1 } else { 0 })
            .collect()
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

    pub fn entropy(&self, frequencies: &[usize]) -> f64 {
        // Shannon entropy formula: H = -sum(p * log2(p))
        let total_weight: usize = self.possible_patterns
            .iter()
            .enumerate()
            .filter(|(_, possible)| **possible)
            .map(|(index, _)| frequencies[index])
            .sum();

        if total_weight == 0 {
            return f64::INFINITY;
        }

        let total_weight = total_weight as f64;

        self.possible_patterns
            .iter()
            .enumerate()
            .filter(|(_, possible)| **possible)
            .map(|(index, _)| {
                let probability = (frequencies[index] as f64) / total_weight;
                -probability * probability.log2()
            })
            .sum()
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
