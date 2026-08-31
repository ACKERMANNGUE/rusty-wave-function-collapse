use std::path::Path;

pub type PatternId = usize;

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

    pub fn get_pixel(&self, x: usize, y: usize) -> Option<&[u8; 4]> {
        if x >= (self.size as usize) || y >= (self.size as usize) {
            return None;
        }

        let index = y * (self.size as usize) + x;
        self.pixels.get(index)
    }

    pub fn extract_pattern(
        image: &image::RgbaImage,
        start_x: u32,
        start_y: u32,
        size: u32,
        id: PatternId
    ) -> Option<Pattern> {
        if start_x + size > image.width() || start_y + size > image.height() {
            return None;
        }
        let mut pixels = Vec::with_capacity((size * size) as usize);

        for y in 0..size {
            for x in 0..size {
                let pixel = image.get_pixel(start_x + x, start_y + y);
                pixels.push([pixel[0], pixel[1], pixel[2], pixel[3]]);
            }
        }

        Some(Pattern::new(id, size, pixels))
    }

    pub fn debug_save_extracted_pattern<P: AsRef<Path>>(&self, path: P) {
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
}
