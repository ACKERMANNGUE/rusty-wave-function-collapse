mod image_manager;
mod pattern;
mod wfc;

use std::path::{ Path, PathBuf };

use crate::{ image_manager::image_io, wfc::{ model::WfcModel, solver::WfcSolver } };

const PATTERN_SIZE: u32 = 4;

const OUTPUT_WAVE_WIDTH: usize = 300;
const OUTPUT_WAVE_HEIGHT: usize = 300;

fn build_input_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("assets/Skyline 2.png")
}

fn build_output_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("assets/output.png")
}

fn main() {
    let input_path = build_input_path();
    let image = image_io::load_image(&input_path).unwrap();
    let model = WfcModel::from_image(&image, PATTERN_SIZE);

    println!("Pattern size: {}", model.get_pattern_size());
    println!("Unique patterns: {}", model.pattern_count());
    println!("Total frequency: {}", model.total_frequency());
    println!("Total adjacency rules: {}", model.get_rules().count_rules());
    println!("Validating adjacency rules...");

    if model.get_rules().validate_rules_symmetry() {
        println!("Adjacency rules are symmetric.");
    } else {
        println!("Adjacency rules are NOT symmetric.");
        return;
    }

    let mut solver = WfcSolver::new(OUTPUT_WAVE_WIDTH, OUTPUT_WAVE_HEIGHT, &model);
    let mut rng = rand::rng();

    println!("Solving wave...");

    let success = solver.solve(&model, &mut rng);

    println!("Solve success: {}", success);
    println!("Wave fully collapsed: {}", solver.get_wave().is_fully_collapsed());
    println!("Wave has contradiction: {}", solver.get_wave().has_contradiction());

    println!(
        "Wave constraints valid: {}",
        solver.get_wave().validate_constraints(model.get_rules())
    );

    if !success {
        println!("Generation failed because of a contradiction.");
        return;
    }

    let Some(output) = wfc::renderer::render_wave(solver.get_wave(), model.get_patterns()) else {
        println!("Cannot render an unresolved wave.");
        return;
    };

    let output_path = build_output_path();
    image_io::save_image(&output, &output_path).unwrap();
    println!("Output saved to: {:?}", output_path);
}
