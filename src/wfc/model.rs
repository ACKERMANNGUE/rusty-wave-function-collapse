use crate::{
    pattern::{ Pattern, PatternId, pattern_extractor::PatternExtractor }, wfc::rules::AdjacencyRules,
};

pub struct WfcModel {
    pattern_size: u32,
    patterns: Vec<Pattern>,
    rules: AdjacencyRules,
    pattern_weights: Vec<u32>,
    pattern_weight_log_weights: Vec<f64>,
    total_weight: u64,
    total_weight_log_weight: f64,
}

impl WfcModel {
    pub fn from_image(image: &image::RgbaImage, pattern_size: u32) -> Self {
        let extractor = PatternExtractor::new(pattern_size);
        let patterns = extractor.extract_unique_patterns(image);
        let mut rules = AdjacencyRules::new(patterns.len());

        let pattern_weights: Vec<u32> = patterns
            .iter()
            .map(|pattern| pattern.get_frequency())
            .collect();

        let pattern_weight_log_weights: Vec<f64> = pattern_weights
            .iter()
            .map(|&weight| {
                let weight = weight as f64;
                weight * weight.ln()
            })
            .collect();

        let total_weight = pattern_weights
            .iter()
            .map(|&weight| weight as u64)
            .sum();

        let total_weight_log_weight: f64 = pattern_weight_log_weights.iter().sum();

        rules.compute_rules(&patterns);

        Self {
            pattern_size,
            patterns,
            rules,
            pattern_weights,
            pattern_weight_log_weights,
            total_weight,
            total_weight_log_weight,
        }
    }

    pub fn pattern_weights(&self) -> &[u32] {
        &self.pattern_weights
    }

    pub fn pattern_weight(&self, pattern_id: PatternId) -> u32 {
        self.pattern_weights[pattern_id]
    }

    pub fn pattern_weight_log_weight(&self, pattern_id: PatternId) -> f64 {
        self.pattern_weight_log_weights[pattern_id]
    }

    pub fn total_weight(&self) -> u64 {
        self.total_weight
    }

    pub fn total_weight_log_weight(&self) -> f64 {
        self.total_weight_log_weight
    }
    pub fn initial_entropy(&self) -> f64 {
        if self.total_weight == 0 || self.pattern_count() <= 1 {
            return 0.0;
        }
        
        // Shannon entropy formula: H = ln(W) - (sum(w_i * ln(w_i)) / W)
        let weight_sum = self.total_weight as f64;
        let entropy = weight_sum.ln() - self.total_weight_log_weight / weight_sum;
        entropy.max(0.0)
    }

    pub fn get_pattern_size(&self) -> u32 {
        self.pattern_size
    }

    pub fn get_patterns(&self) -> &[Pattern] {
        &self.patterns
    }

    pub fn get_rules(&self) -> &AdjacencyRules {
        &self.rules
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    pub fn total_frequency(&self) -> u32 {
        self.patterns
            .iter()
            .map(|pattern| pattern.get_frequency())
            .sum()
    }
}
