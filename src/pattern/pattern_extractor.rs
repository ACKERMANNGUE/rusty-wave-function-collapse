use crate::pattern::{ Pattern, PatternId };

use std::collections::HashMap;

pub struct PatternExtractor {
    pattern_size: u32,
}

impl PatternExtractor {
    pub fn new(pattern_size: u32) -> Self {
        Self { pattern_size }
    }

    fn extract_pattern(
        &self,
        image: &image::RgbaImage,
        start_x: u32,
        start_y: u32,
        id: PatternId
    ) -> Option<Pattern> {
        if
            start_x + self.pattern_size > image.width() ||
            start_y + self.pattern_size > image.height()
        {
            return None;
        }
        let mut pixels = Vec::with_capacity((self.pattern_size * self.pattern_size) as usize);

        for y in 0..self.pattern_size {
            for x in 0..self.pattern_size {
                let pixel = image.get_pixel(start_x + x, start_y + y);
                pixels.push([pixel[0], pixel[1], pixel[2], pixel[3]]);
            }
        }

        Some(Pattern::new(id, self.pattern_size, pixels))
    }

    pub fn extract_all_patterns(&self, image: &image::RgbaImage) -> Vec<Pattern> {
        if image.width() < self.pattern_size || image.height() < self.pattern_size {
            println!("Image is smaller than the pattern size. No patterns can be extracted.");
            return Vec::new();
        }

        let mut patterns: Vec<Pattern> = Vec::new();
        let mut id_counter: PatternId = 0;

        let max_x = image.width() - self.pattern_size;
        let max_y = image.height() - self.pattern_size;

        for y in 0..=max_y {
            for x in 0..=max_x {
                if let Some(pattern) = self.extract_pattern(image, x, y, id_counter) {
                    patterns.push(pattern);
                    id_counter += 1;
                }
            }
        }

        patterns
    }

    pub fn extract_unique_patterns(&self, image: &image::RgbaImage) -> Vec<Pattern> {
        if image.width() < self.pattern_size || image.height() < self.pattern_size {
            println!("Image is smaller than the pattern size. No patterns can be extracted.");

            return Vec::new();
        }

        let mut unique_patterns: Vec<Pattern> = Vec::new();
        let mut pattern_lookup: HashMap<u64, Vec<PatternId>> = HashMap::new(); // HashMap to store pattern hashes and their corresponding IDs, it's easier to check for duplicates using hashes

        let max_x = image.width() - self.pattern_size;
        let max_y = image.height() - self.pattern_size;

        for y in 0..=max_y {
            for x in 0..=max_x {
                let candidate_id = unique_patterns.len(); // Use the current length of unique_patterns as the candidate ID for the new pattern
                let Some(pattern) = self.extract_pattern(image, x, y, candidate_id) else {
                    continue;
                };

                let hash = pattern.compute_hash();
                let mut duplicate_id: Option<PatternId> = None;

                if let Some(pattern_ids) = pattern_lookup.get(&hash) {
                    for pattern_id in pattern_ids {
                        let existing_pattern = &unique_patterns[*pattern_id];

                        if pattern.has_same_pixels(existing_pattern) {
                            duplicate_id = Some(*pattern_id);
                            break;
                        }
                    }
                }

                if let Some(pattern_id) = duplicate_id {
                    unique_patterns[pattern_id].increment_frequency();
                } else {
                    let pattern_id = pattern.get_id();
                    unique_patterns.push(pattern);
                    pattern_lookup.entry(hash).or_default().push(pattern_id);
                }
            }
        }

        unique_patterns
    }
}
