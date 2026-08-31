mod image_manager;
mod pattern;
mod wfc;

use std::path::{ Path, PathBuf };

use crate::{ image_manager::image_io, wfc::{ model::WfcModel, solver::WfcSolver } };

const PATTERN_SIZE: u32 = 4;

const OUTPUT_WAVE_WIDTH: usize = 300;
const OUTPUT_WAVE_HEIGHT: usize = 300;

const MAX_ATTEMPTS: usize = 100;

fn build_input_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest_dir).join("assets/input.png")
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

    let mut rng = rand::rng();

    let mut time = std::time::Instant::now();
    let overall_time = std::time::Instant::now();

    for attempt in 1..=MAX_ATTEMPTS {
        println!();
        println!("Generation attempt {}/{}", attempt, MAX_ATTEMPTS);
        println!("Started at: {:?}", time);

        let mut solver = WfcSolver::new(OUTPUT_WAVE_WIDTH, OUTPUT_WAVE_HEIGHT, &model);
        println!("Solving wave...");
        println!("Parameters:");
        println!("Wave width: {}", solver.get_wave().get_width());
        println!("Wave height: {}", solver.get_wave().get_height());

        let success = solver.solve(&model, &mut rng);

        println!("Solve success: {}", success);
        println!("Wave fully collapsed: {}", solver.get_wave().is_fully_collapsed());
        println!("Wave has contradiction: {}", solver.get_wave().has_contradiction());

        if !success {
            println!("Contradiction detected. Restarting generation...");
            time = std::time::Instant::now();
            continue;
        }

        let constraints_valid = solver.get_wave().validate_constraints(model.get_rules());
        println!("Wave constraints valid: {}", constraints_valid);

        if !constraints_valid {
            println!("Invalid final constraints. Restarting generation...");
            time = std::time::Instant::now();
            continue;
        }

        let Some(output) = wfc::renderer::render_wave(
            solver.get_wave(),
            model.get_patterns()
        ) else {
            println!("Cannot render the resolved wave. Restarting generation...");
            time = std::time::Instant::now();
            continue;
        };

        let output_path = build_output_path();
        image_io::save_image(&output, &output_path).unwrap();
        println!("Output saved to: {:?}", output_path);
        println!("Time taken: {:?}", time.elapsed());
        println!("Overall time taken: {:?}", overall_time.elapsed());
        return;
    }

    println!();
    println!("Generation failed after {} attempts.", MAX_ATTEMPTS);
}
