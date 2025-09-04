//! Dive Computer Decompression Planner
//!
//! This example demonstrates the built-in dive-computer-deco library functionality
//! for decompression calculations.
//!
//! For comparison with an external reference implementation, see:
//! `dive_deco_planner.rs` - Uses the external dive-deco crate
//!
//! Run with: `cargo run --example planner`

use dive_computer_deco::{
    DiveParameters, 
    tissue::Tissue, 
    default_tissue_load,
    ndl::ndl,
    ceiling::max_ceiling,
    simulate::{simulate, simulate_with_ascent},
};
use std::io::{self, Write};

fn get_float_input(prompt: &str, default: f32) -> f32 {
    loop {
        print!("{} (default: {}): ", prompt, default);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let input = input.trim();
        if input.is_empty() {
            return default;
        }

        match input.parse::<f32>() {
            Ok(value) => return value,
            Err(_) => println!("Invalid input. Please enter a valid number."),
        }
    }
}

fn validate_gradient_factors(gf_high: f32, gf_low: f32) -> (f32, f32) {
    let mut validated_gf_high = gf_high;
    let mut validated_gf_low = gf_low;
    
    // Warn about extreme gradient factor values
    if gf_high >= 1.0 || gf_low >= 1.0 {
        println!("⚠️  WARNING: Gradient factors of 1.0 or higher can cause mathematical instability!");
        println!("   This may result in infinite loops or unrealistic decompression calculations.");
        println!("   Clamping gradient factors to maximum safe value of 0.99.");
        
        if validated_gf_high >= 1.0 {
            validated_gf_high = 0.99;
        }
        if validated_gf_low >= 1.0 {
            validated_gf_low = 0.99;
        }
    }
    
    // Ensure GF Low <= GF High
    if validated_gf_low > validated_gf_high {
        println!("⚠️  WARNING: GF Low should not be higher than GF High. Swapping values.");
        let temp = validated_gf_high;
        validated_gf_high = validated_gf_low;
        validated_gf_low = temp;
    }
    
    // Warn about very conservative values
    if validated_gf_high < 0.3 || validated_gf_low < 0.2 {
        println!("⚠️  WARNING: Very conservative gradient factors may result in excessive decompression times.");
    }
    
    (validated_gf_high, validated_gf_low)
}

fn get_depths_input() -> Vec<f32> {
    loop {
        print!("Enter dive depths in meters (comma-separated, e.g., 18,30,40): ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let input = input.trim();
        if input.is_empty() {
            return vec![18.0, 30.0, 40.0]; // default values
        }

        let depths: Result<Vec<f32>, _> = input
            .split(',')
            .map(|s| s.trim().parse::<f32>())
            .collect();

        match depths {
            Ok(depths) if !depths.is_empty() => return depths,
            _ => println!("Invalid input. Please enter comma-separated numbers (e.g., 18,30,40)."),
        }
    }
}

fn main() {
    println!("=== Dive Computer Decompression Planner ===\n");

    // Get dive parameters from user input
    println!("Enter dive parameters:");
    let gf_high_input = get_float_input("GF High (0.0-1.0)", 0.8);
    let gf_low_input = get_float_input("GF Low (0.0-1.0)", 0.8);
    let surface_pressure = get_float_input("Surface pressure (bar)", 1.0);
    let temperature = get_float_input("Water temperature (°C)", 37.0);

    // Validate and adjust gradient factors if necessary
    let (gf_high, gf_low) = validate_gradient_factors(gf_high_input, gf_low_input);

    // Initialize dive parameters with validated input
    let mut dive_params = DiveParameters::new(gf_high, gf_low);

    // Initialize tissues with user-specified conditions
    let mut tissues = initialize_tissues(surface_pressure, temperature);

    println!("Dive Parameters:");
    println!("  Descent Speed: {:.2} m/s", dive_params.descent_speed);
    println!("  Ascent Speed: {:.2} m/s", dive_params.ascent_speed);
    println!("  GF Low: {:.0}%", dive_params.gf_low * 100.0);
    println!("  GF High: {:.0}%", dive_params.gf_high * 100.0);
    println!();

    // // Get dive depths from user input
    // println!("\nDive Planning:");
    // let dive_depths = get_depths_input();

    // for &depth in &dive_depths {
    //     plan_dive(depth, &dive_params, &mut tissues.clone(), temperature, surface_pressure);
    //     println!();
    // }

    // Get simulation parameters from user input
    println!("\nDive Simulation:");
    let target_depth = get_float_input("Target depth for simulation (m)", 50.0);
    let bottom_time_minutes = get_float_input("Bottom time for simulation (minutes)", 20.0);
    let interval_seconds = get_float_input("Recording interval (seconds)", 1.0);

    // Demonstrate a dive simulation
    demonstrate_dive_simulation(&mut dive_params, &mut tissues, temperature, surface_pressure, target_depth, bottom_time_minutes, interval_seconds);
}

fn initialize_tissues(surface_pressure: f32, temperature: f32) -> [Tissue; 16] {
    let mut tissues = [Tissue::default(); 16];
    let initial_n2_load = default_tissue_load(temperature);

    // Initialize all tissues with surface nitrogen loading
    for tissue in &mut tissues {
        tissue.load_n2 = initial_n2_load;
        tissue.load_he = 0.0; // No helium at surface
    }

    tissues
}

fn plan_dive(depth: f32, dive_params: &DiveParameters, tissues: &mut [Tissue; 16], temperature: f32, surface_pressure: f32) {
    let pressure_at_depth = depth / 10.0 + 1.0; // Convert depth to pressure (bar)

    println!("=== Planning dive to {:.0}m ({:.1} bar) ===", depth, pressure_at_depth);

    // Calculate No Decompression Limit (NDL)
    let ndl_minutes = ndl(*dive_params, tissues, pressure_at_depth, temperature);
    println!("No Decompression Limit: {:.1} minutes", ndl_minutes);

    // Check current ceiling (should be 0 at surface)
    let (ceiling_depth, controlling_tissue) = max_ceiling(*dive_params, tissues);
    println!("Current ceiling: {}m (controlled by tissue {})", ceiling_depth, controlling_tissue);

    // Simulate different bottom times
    let bottom_times = [ndl_minutes * 0.5, ndl_minutes * 0.8, ndl_minutes * 1.2];

    for &bottom_time in &bottom_times {
        println!("\n  Bottom time: {:.1} minutes", bottom_time);

        // Create a copy of tissues for simulation
        let mut sim_tissues = *tissues;

        println!("    Simulating dive...");
        
        // Simulate the dive
        let _outputs = simulate(
            &mut dive_params.clone(),
            &mut sim_tissues,
            surface_pressure,
            depth,
            temperature,
            60.0, // 60 second intervals
            bottom_time * 60.0, // convert minutes to seconds
        );

        // Check ceiling after the dive
        let (post_dive_ceiling, controlling_tissue) = max_ceiling(*dive_params, &sim_tissues);

        if post_dive_ceiling == 0 {
            println!("    Result: No decompression required");
        } else {
            println!("    Result: Decompression required - ceiling at {}m (tissue {})", 
                    post_dive_ceiling, controlling_tissue);
        }

        // Show tissue loading for the most loaded tissue
        let most_loaded_tissue = &sim_tissues[controlling_tissue];
        println!("    Controlling tissue N2: {:.3} bar, He: {:.3} bar", 
                most_loaded_tissue.load_n2, most_loaded_tissue.load_he);
    }
}

fn demonstrate_dive_simulation(dive_params: &mut DiveParameters, tissues: &mut [Tissue; 16], temperature: f32, surface_pressure: f32, target_depth: f32, bottom_time_minutes: f32, interval_seconds: f32) {
    println!("=== Dive Simulation Example ===");
    println!("Simulating dive to {:.0}m for {:.1} minutes...", target_depth, bottom_time_minutes);

    // Reset tissues to surface conditions
    *tissues = initialize_tissues(surface_pressure, temperature);

    let outputs = simulate_with_ascent(
        dive_params,
        tissues,
        surface_pressure,
        target_depth,
        temperature,
        interval_seconds,
        bottom_time_minutes * 60.0, // convert to seconds
        true, // include ascent with decompression
    );

    // Check final state
    let (final_ceiling, controlling_tissue) = max_ceiling(*dive_params, tissues);

    println!("Simulation complete!");
    println!("Final ceiling: {}m (controlled by tissue {})", final_ceiling, controlling_tissue);

    if final_ceiling > 0 {
        println!("⚠️  Warning: Tissues still show decompression obligation after simulation!");
        println!("This may indicate incomplete decompression or very conservative settings.");

        // Show tissue loadings
        println!("\nFinal tissue loadings:");
        for (i, tissue) in tissues.iter().enumerate() {
            if tissue.load_n2 > 1.2 || tissue.load_he > 0.05 { // Only show significantly loaded tissues
                println!("  Tissue {}: N2={:.3} bar, He={:.3} bar", i, tissue.load_n2, tissue.load_he);
            }
        }
    } else {
        println!("✅ Successful ascent to surface - all tissues cleared");
    }

    // Show simulation details - only available with serde feature
    #[cfg(feature = "serde")]
    {
        println!("\nSimulation recorded {} data points", outputs.depths.len());
        if !outputs.depths.is_empty() {
            let max_depth = outputs.depths.iter().fold(0.0f32, |a, &b| a.max(b));
            let min_depth = outputs.depths.iter().fold(f32::INFINITY, |a, &b| a.min(b));
            println!("Max depth reached: {:.1}m", max_depth);
            println!("Final depth: {:.1}m", min_depth);
            
            // Calculate total dive time
            let total_time = outputs.depths.len() as f32 * interval_seconds / 60.0;
            println!("Total simulation time: {:.1} minutes", total_time);
            
            // Analyze and print decompression stops
            analyze_decompression_stops(&outputs.depths, interval_seconds);
        }
    }

    #[cfg(not(feature = "serde"))]
    {
        println!("\n⚠️  Note: Detailed decompression stop analysis requires the 'serde' feature.");
        println!("   Run with: cargo run --features serde --example planner");
        println!("   The simulation does include decompression stops, but they're not recorded for analysis.");
    }
}

fn analyze_decompression_stops(depths: &[f32], interval_seconds: f32) {
    println!("\n=== Decompression Stop Analysis ===");
    
    if depths.len() < 2 {
        println!("Insufficient data for stop analysis");
        return;
    }
    
    let max_depth = depths.iter().fold(0.0f32, |a, &b| a.max(b));
    let mut stops = Vec::new();
    let mut current_stop_depth = None;
    let mut stop_start_time = 0.0;
    let mut ascending = false;
    
    // Track when we start ascending from the bottom
    let mut bottom_phase_ended = false;
    
    // Find when we actually reach maximum depth (or close to it) first
    let mut reached_max_depth = false;
    
    for (i, &depth) in depths.iter().enumerate() {
        let time_minutes = i as f32 * interval_seconds / 60.0;
        
        // First, check if we've reached near the maximum depth
        if !reached_max_depth && depth >= max_depth * 0.95 {
            reached_max_depth = true;
        }
        
        // Only start looking for ascent after we've reached the maximum depth
        if reached_max_depth && !bottom_phase_ended && depth < max_depth * 0.95 {
            bottom_phase_ended = true;
            ascending = true;
            println!("Ascent phase starts at {:.1} minutes", time_minutes);
        }
        
        if !bottom_phase_ended {
            continue; // Still in descent/bottom phase
        }
        
        if i == 0 {
            continue;
        }
        
        let prev_depth = depths[i - 1];
        
        // Detect start of a stop (depth stays the same while ascending)
        if current_stop_depth.is_none() && depth == prev_depth && depth > 0.5 && ascending {
            current_stop_depth = Some(depth);
            stop_start_time = time_minutes;
        }
        // Detect end of a stop (depth changes)
        else if let Some(stop_depth) = current_stop_depth {
            if depth != stop_depth {
                let stop_duration = time_minutes - stop_start_time;
                // Only record stops that last at least one interval and are at least 1m deep
                if stop_duration >= interval_seconds / 60.0 && stop_depth >= 1.0 {
                    stops.push((stop_depth, stop_duration, stop_start_time));
                }
                current_stop_depth = None;
            }
        }
    }
    
    // Handle case where simulation ends during a stop
    if let Some(stop_depth) = current_stop_depth {
        let final_time = (depths.len() - 1) as f32 * interval_seconds / 60.0;
        let stop_duration = final_time - stop_start_time;
        if stop_duration >= interval_seconds / 60.0 && stop_depth >= 1.0 {
            stops.push((stop_depth, stop_duration, stop_start_time));
        }
    }
    
    if stops.is_empty() {
        println!("No decompression stops detected");
    } else {
        println!("Decompression stops found:");
        println!("Depth (m) | Duration (secs) | Duration (min) | Start Time (min) | End Time (min)");
        println!("----------|----------------|----------------|------------------|----------------");
        
        let total_deco_time: f32 = stops.iter().map(|(_, duration, _)| duration).sum();
        let num_stops = stops.len();
        
        for (depth, duration, start_time) in &stops {
            println!("   {:4.1}   |     {:6.1}     |     {:6.1}     |      {:6.1}      |      {:6.1}", depth, duration * 60., duration, start_time, start_time + duration);
        }
        
        println!("\nTotal decompression time: {:.1} minutes", total_deco_time);
        println!("Number of stops: {}", num_stops);
        
        // Show the deepest and shallowest stops
        if let Some((deepest_depth, _, _)) = stops.iter().max_by(|a, b| a.0.partial_cmp(&b.0).unwrap()) {
            println!("Deepest stop: {:.1}m", deepest_depth);
        }
        if let Some((shallowest_depth, _, _)) = stops.iter().min_by(|a, b| a.0.partial_cmp(&b.0).unwrap()) {
            println!("Shallowest stop: {:.1}m", shallowest_depth);
        }
    }
}
