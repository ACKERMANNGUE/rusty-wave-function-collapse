mod image_manager;
mod pattern;
mod wfc;

use image_manager::image_io;

use std::path::{ Path, PathBuf };

use crate::{pattern::pattern_extractor::PatternExtractor, wfc::rules::AdjacencyRules};

fn build_input_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("assets/input.png")
}

fn build_output_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("assets/patterns")
}

fn main() {
    let input_path = build_input_path();
    let output_path = build_output_path();

    let image = image_io::load_image(&input_path).unwrap();
    let extractor = PatternExtractor::new(2);
    let patterns = extractor.extract_unique_patterns(&image);
    println!("Unique patterns: {}", patterns.len());
    let total_frequency: u32 = patterns
        .iter()
        .map(|pattern| pattern.get_frequency())
        .sum();

    println!("Total frequency: {}", total_frequency);
    let mut rules = AdjacencyRules::new(patterns.len());
    rules.compute_rules(&patterns);
    println!("Total adjacency rules: {}", rules.count_rules());
    println!("Validating adjacency rules...");
    if rules.validate_rules_symmetry() {
        println!("Adjacency rules are symmetric.");
    } else {
        println!("Adjacency rules are NOT symmetric.");
    }
    // save_extracted_patterns_individually(&patterns, &output_path);
}

fn save_extracted_patterns_individually(patterns: &[pattern::Pattern], output_dir: &Path) {
    for pattern in patterns {
        let pattern_output_path = output_dir.join(format!("pattern_{}.png", pattern.id));
        pattern.save_to_image(&pattern_output_path);
    }
}
