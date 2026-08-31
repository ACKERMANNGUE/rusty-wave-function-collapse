mod image_manager;
mod pattern;

use image_manager::image_io;
use pattern::Pattern;

use std::path::{ Path, PathBuf };

const PATH_OUTPUT: &str = "assets/output.png";

fn build_input_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("assets/input.png")
}

fn build_output_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join(PATH_OUTPUT)
}

fn main() {
    let input_path = build_input_path();
    let output_path = build_output_path();

    let image = image_io::load_image(&input_path).unwrap();
    let pattern = Pattern::extract_pattern(&image, 0, 0, 32, 1).unwrap();
    pattern.debug_save_extracted_pattern(&output_path);
}
