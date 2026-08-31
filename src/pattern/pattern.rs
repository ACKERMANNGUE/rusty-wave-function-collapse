use std::path::Path;

pub type PatternId = usize;

use std::collections::hash_map::DefaultHasher;
use std::hash::{ Hash, Hasher };

pub struct Pattern {
    pub id: PatternId,
    pub size: u32,
    pub pixels: Vec<[u8; 4]>,
    pub frequency: u32,
}

impl Pattern {
    pub fn new(id: PatternId, size: u32, pixels: Vec<[u8; 4]>) -> Self {
        assert_eq!(
            pixels.len(),
            (size * size) as usize,
            "Pixels length does not match size squared"
        );

        Self {
            id,
            size,
            pixels,
            frequency: 1,
        }
    }

    pub fn increment_frequency(&mut self) {
        self.frequency += 1;
    }

    pub fn get_id(&self) -> PatternId {
        self.id
    }

    pub fn get_size(&self) -> u32 {
        self.size
    }

    pub fn get_frequency(&self) -> u32 {
        self.frequency
    }

    pub fn get_pixels(&self) -> &[[u8; 4]] {
        &self.pixels
    }

    pub fn save_to_image(&self, path: &Path) {
        let mut image = image::RgbaImage::new(self.size, self.size);

        for y in 0..self.size {
            for x in 0..self.size {
                if let Some(pixel) = self.get_pixel(x as usize, y as usize) {
                    image.put_pixel(x, y, image::Rgba(*pixel));
                }
            }
        }

        image.save(path).unwrap();
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Option<&[u8; 4]> {
        if x >= (self.size as usize) || y >= (self.size as usize) {
            return None;
        }

        let index = y * (self.size as usize) + x;
        self.pixels.get(index)
    }

    pub fn has_same_pixels(&self, other: &Pattern) -> bool {
        self.size == other.size && self.pixels == other.pixels
    }

    pub fn compute_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();

        self.size.hash(&mut hasher);
        self.pixels.hash(&mut hasher);

        hasher.finish()
    }

    pub fn set_id(&mut self, new_id: PatternId) {
        self.id = new_id;
    }
}
