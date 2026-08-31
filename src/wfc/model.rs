use crate::{
    pattern::{ pattern_extractor::PatternExtractor, Pattern },
    wfc::rules::AdjacencyRules,
};

pub struct WfcModel {
    pattern_size: u32,
    patterns: Vec<Pattern>,
    rules: AdjacencyRules,
}

impl WfcModel {
    pub fn from_image(image: &image::RgbaImage, pattern_size: u32) -> Self {
        let extractor = PatternExtractor::new(pattern_size);
        let patterns = extractor.extract_unique_patterns(image);
        let mut rules = AdjacencyRules::new(patterns.len());
        rules.compute_rules(&patterns);

        Self {
            pattern_size,
            patterns,
            rules,
        }
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
