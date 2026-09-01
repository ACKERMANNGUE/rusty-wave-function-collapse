mod image_manager;
mod pattern;
mod wfc;

use std::{ path::{ Path, PathBuf }, time::{ Duration, Instant } };

use crate::{ image_manager::image_io, wfc::{ model::WfcModel, solver::WfcSolver } };

const PATTERN_SIZE: u32 = 2;

const OUTPUT_WAVE_WIDTH: usize = 64 * 2;
const OUTPUT_WAVE_HEIGHT: usize = 64 * 2;

const MAX_ATTEMPTS: usize = 100000;

fn build_input_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    Path::new(manifest_dir).join("assets/input.png")
}

fn build_output_path() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    Path::new(manifest_dir).join("assets/output.png")
}

fn print_duration(name: &str, duration: Duration) {
    println!("{:<30} {:>12.3} ms", name, duration.as_secs_f64() * 1000.0);
}

fn main() {
    let program_start = Instant::now();

    println!("=== WFC PERFORMANCE REPORT ===");
    println!();

    let input_path = build_input_path();
    let image_load_start = Instant::now();
    let image = image_io::load_image(&input_path).unwrap();
    let image_load_duration = image_load_start.elapsed();

    println!("Input:");
    println!("  Path: {:?}", input_path);
    println!("  Dimensions: {}x{}", image.width(), image.height());
    println!();
    print_duration("Image loading", image_load_duration);

    let model_start = Instant::now();
    let model = WfcModel::from_image(&image, PATTERN_SIZE);
    let model_duration = model_start.elapsed();

    println!();
    println!("Model:");
    println!("  Pattern size: {}", model.get_pattern_size());
    println!("  Unique patterns: {}", model.pattern_count());
    println!("  Total frequency: {}", model.total_frequency());
    println!("  Adjacency rules: {}", model.get_rules().count_rules());
    println!();
    print_duration("Model creation", model_duration);
    println!();
    println!("Wave:");
    println!("  Width: {}", OUTPUT_WAVE_WIDTH);
    println!("  Height: {}", OUTPUT_WAVE_HEIGHT);
    println!("  Cells: {}", OUTPUT_WAVE_WIDTH * OUTPUT_WAVE_HEIGHT);
    println!("  Max attempts: {}", MAX_ATTEMPTS);

    let mut rng = rand::rng();

    let generation_start = Instant::now();

    let mut total_solve_duration = Duration::ZERO;
    let mut total_solver_creation_duration = Duration::ZERO;

    for attempt in 1..=MAX_ATTEMPTS {
        println!();
        println!("=== ATTEMPT {}/{} ===", attempt, MAX_ATTEMPTS);
        let attempt_start = Instant::now();

        let solver_creation_start = Instant::now();
        let mut solver = WfcSolver::new(OUTPUT_WAVE_WIDTH, OUTPUT_WAVE_HEIGHT, &model);
        let solver_creation_duration = solver_creation_start.elapsed();
        total_solver_creation_duration += solver_creation_duration;

        let solve_start = Instant::now();
        let success = solver.solve(&model, &mut rng);
        let solve_duration = solve_start.elapsed();
        total_solve_duration += solve_duration;

        print_duration("Solver creation", solver_creation_duration);
        print_duration("Solve", solve_duration);
        println!("{:<30} {}", "Solve success", success);
        println!("{:<30} {}", "Fully collapsed", solver.get_wave().is_fully_collapsed());
        println!("{:<30} {}", "Has contradiction", solver.get_wave().has_contradiction());
        println!("{:<30} {}", "Unresolved cells", solver.get_wave().get_unresolved_count());
        println!("{:<30} {}", "Contradiction cells", solver.get_wave().get_contradiction_count());

        let solver_timings = solver.get_timings();
        println!("Observe total:     {:.3} ms", solver_timings.observe.as_secs_f64() * 1000.0);
        println!("Propagate total:   {:.3} ms", solver_timings.propagate.as_secs_f64() * 1000.0);
        println!("Observations:      {}", solver_timings.observations);
        println!("Propagation calls: {}", solver_timings.propagation_calls);
        let stats = solver.get_propagation_stats();
        println!("Propagation stats:");
        println!("  Queue pops:         {}", stats.queue_pops);
        println!("  Neighbor checks:    {}", stats.neighbor_checks);
        println!("  Collapsed current:  {}", stats.collapsed_current);
        println!("  Collapsed neighbor: {}", stats.collapsed_neighbor);
        println!("  Changed neighbors:  {}", stats.changed_neighbors);

        if !success {
            let attempt_duration = attempt_start.elapsed();
            println!("Contradiction detected. Restarting generation...");
            print_duration("Attempt total", attempt_duration);
            continue;
        }

        let validation_start = Instant::now();
        let constraints_valid = solver.get_wave().validate_constraints(model.get_rules());
        let validation_duration = validation_start.elapsed();
        print_duration("Constraint validation", validation_duration);
        println!("{:<30} {}", "Constraints valid", constraints_valid);

        if !constraints_valid {
            let attempt_duration = attempt_start.elapsed();
            println!("Invalid final constraints. Restarting generation...");
            print_duration("Attempt total", attempt_duration);
            continue;
        }

        let render_start = Instant::now();
        let Some(output) = wfc::renderer::render_wave(
            solver.get_wave(),
            model.get_patterns()
        ) else {
            let render_duration = render_start.elapsed();
            print_duration("Render", render_duration);
            println!("Cannot render the resolved wave. Restarting generation...");
            continue;
        };

        let render_duration = render_start.elapsed();
        print_duration("Render", render_duration);
        println!("{:<30} {}x{}", "Output dimensions", output.width(), output.height());

        let output_path = build_output_path();
        let save_start = Instant::now();
        image_io::save_image(&output, &output_path).unwrap();
        let save_duration = save_start.elapsed();
        print_duration("Image saving", save_duration);

        let attempt_duration = attempt_start.elapsed();
        let generation_duration = generation_start.elapsed();
        let program_duration = program_start.elapsed();

        println!();
        println!("=== SUCCESS ===");
        println!("Output saved to: {:?}", output_path);
        println!();
        println!("Timings:");
        print_duration("Image loading", image_load_duration);
        print_duration("Model creation", model_duration);
        print_duration("Total solver creation", total_solver_creation_duration);
        print_duration("Total solve", total_solve_duration);
        print_duration("Successful attempt", attempt_duration);
        print_duration("Generation total", generation_duration);
        print_duration("Program total", program_duration);
        println!();
        println!("Attempts required: {}", attempt);

        return;
    }

    println!();
    println!("=== FAILURE ===");
    println!("Generation failed after {} attempts.", MAX_ATTEMPTS);
    println!();
    print_duration("Total solver creation", total_solver_creation_duration);
    print_duration("Total solve", total_solve_duration);
    print_duration("Generation total", generation_start.elapsed());
    print_duration("Program total", program_start.elapsed());
}
