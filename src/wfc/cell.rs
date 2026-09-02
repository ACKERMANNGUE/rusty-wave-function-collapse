use crate::{ pattern::PatternId, wfc::bitset::BitSet };

pub struct Cell {
    possible_patterns: BitSet,
    possible_count: usize,
    weight_sum: u64,
    weight_log_weight_sum: f64,
}

impl Cell {
    pub fn new(pattern_count: usize, total_weight: u64, total_weight_log_weight: f64) -> Self {
        Self {
            possible_patterns: BitSet::full(pattern_count),
            possible_count: pattern_count,
            weight_sum: total_weight,
            weight_log_weight_sum: total_weight_log_weight,
        }
    }

    pub fn possible_pattern_words(&self) -> &[u64] {
        self.possible_patterns.words()
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

    pub fn remove_pattern(
        &mut self,
        pattern_id: PatternId,
        weight: u32,
        weight_log_weight: f64
    ) -> bool {
        if !self.possible_patterns.remove(pattern_id) {
            return false;
        }

        self.possible_count -= 1;
        self.weight_sum = self.weight_sum
            .checked_sub(weight as u64)
            .expect("Cell weight sum underflow");
        self.weight_log_weight_sum -= weight_log_weight;

        if self.possible_count == 0 {
            self.weight_sum = 0;
            self.weight_log_weight_sum = 0.0;
        }

        true
    }

    pub fn restore_pattern(
        &mut self,
        pattern_id: PatternId,
        weight: u32,
        weight_log_weight: f64
    ) -> bool {
        if !self.possible_patterns.insert(pattern_id) {
            return false;
        }

        self.possible_count += 1;
        self.weight_sum += weight as u64;
        self.weight_log_weight_sum += weight_log_weight;

        true
    }

    pub fn weight_sum(&self) -> u64 {
        self.weight_sum
    }

    pub fn entropy(&self) -> f64 {
        if self.possible_count <= 1 || self.weight_sum == 0 {
            return 0.0;
        }
        // Shannon entropy formula: H = ln(W) - (sum(w_i * ln(w_i)) / W)
        let weight_sum = self.weight_sum as f64;
        let entropy = weight_sum.ln() - self.weight_log_weight_sum / weight_sum;
        entropy.max(0.0)
    }

    pub fn possible_pattern_ids(&self) -> impl Iterator<Item = PatternId> + '_ {
        self.possible_patterns.iter_ones()
    }

    pub fn possible_count(&self) -> usize {
        self.possible_count
    }

    pub fn collapsed_pattern_id(&self) -> Option<PatternId> {
        if !self.is_collapsed() {
            return None;
        }

        self.possible_patterns.iter_ones().next()
    }
}
