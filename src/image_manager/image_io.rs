use std::path::Path;

use image::{ImageReader, ImageResult, RgbaImage};

pub fn load_image<P: AsRef<Path>>(path: P) -> ImageResult<RgbaImage> {
    let image = ImageReader::open(path)?
        .decode()?
        .to_rgba8();

    Ok(image)
}

pub fn save_image<P: AsRef<Path>>(
    image: &RgbaImage,
    path: P,
) -> ImageResult<()> {
    image.save(path)?;

    Ok(())
}