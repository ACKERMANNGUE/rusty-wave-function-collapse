mod image_manager;
use image_manager::image_io;
use std::path::{Path, PathBuf};

const PATH_OUTPUT: &str = "assets/output.png";

fn build_input_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let input_path = Path::new(manifest_dir).join("assets/input.png");
    input_path
}

fn build_output_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let output_path = Path::new(manifest_dir).join(PATH_OUTPUT);
    output_path
}

fn main() {
    let input_path = build_input_path();
    let image = image_io::load_image(&input_path).unwrap();
    let output_path = build_output_path();
    image_io::save_image(&image, &output_path).unwrap();
}
