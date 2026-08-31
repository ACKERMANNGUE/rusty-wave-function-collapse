pub type PatternId = usize;

use std::collections::hash_map::DefaultHasher;
use std::hash::{ Hash, Hasher };

use crate::wfc::direction::Direction;

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

    pub fn overlaps(&self, other: &Pattern, direction: Direction) -> bool {
        if self.size != other.size {
            println!(
                "Patterns have different sizes: self.size = {}, other.size = {}",
                self.size,
                other.size
            );
            return false;
        }

        if self.size < 2 {
            println!("Pattern size is too small for overlap check: size = {}", self.size);
            return false;
        }

        let size = self.size as usize; // converts size to usize for indexing

        match direction {
            Direction::Right => {
                for y in 0..size {
                    for x in 0..size - 1 {
                        let current_pixel = self.get_pixel(x + 1, y).unwrap();
                        let other_pixel = other.get_pixel(x, y).unwrap();
                        if current_pixel != other_pixel {
                            return false;
                        }
                    }
                }
            }
            Direction::Left => {
                for y in 0..size {
                    for x in 0..size - 1 {
                        let current_pixel = self.get_pixel(x, y).unwrap();
                        let other_pixel = other.get_pixel(x + 1, y).unwrap();
                        if current_pixel != other_pixel {
                            return false;
                        }
                    }
                }
            }
            Direction::Down => {
                for y in 0..size - 1 {
                    for x in 0..size {
                        let current_pixel = self.get_pixel(x, y + 1).unwrap();
                        let other_pixel = other.get_pixel(x, y).unwrap();
                        if current_pixel != other_pixel {
                            return false;
                        }
                    }
                }
            }
            Direction::Up => {
                for y in 0..size - 1 {
                    for x in 0..size {
                        let current_pixel = self.get_pixel(x, y).unwrap();
                        let other_pixel = other.get_pixel(x, y + 1).unwrap();
                        if current_pixel != other_pixel {
                            return false;
                        }
                    }
                }
            }
        }

        true
    }
}
