//! Dive Computer Decompression Planner using the dive-deco library
//!
//! This example demonstrates how to use the external `dive-deco` library for
//! decompression calculations, providing an alternative implementation to the
//! built-in dive-computer-deco library.
//!
//! Key differences from the main planner:
//! - Uses the dive-deco crate's BuehlmannModel
//! - Different API for recording dives and calculating NDL/ceiling
//! - May have different gradient factor handling
//! - Provides a reference implementation for comparison
//!
//! Run with: `cargo run --example dive_deco_planner`

use dive_deco::{
    BuehlmannModel, 
    DecoModel, 
    Depth, 
    Gas, 
    Time
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

fn main() {
    println!("=== Dive-Deco Library Decompression Planner ===");
    println!("Using external dive-deco crate as reference implementation");
    println!("Compare results with the main planner.rs example\n");

    // Get dive parameters from user input
    println!("Enter dive parameters:");
    let gf_high_input = get_float_input("GF High (0.0-1.0)", 0.8);
    let gf_low_input = get_float_input("GF Low (0.0-1.0)", 0.80);
    let surface_pressure = get_float_input("Surface pressure (bar)", 1.0);
    let temperature = get_float_input("Water temperature (°C)", 37.0);

    // Validate and adjust gradient factors if necessary
    let (gf_high, gf_low) = validate_gradient_factors(gf_high_input, gf_low_input);

    // Note: dive-deco library doesn't expose descent/ascent speeds in the same way
    println!("Dive Parameters:");
    println!("  GF Low: {:.0}%", gf_low * 100.0);
    println!("  GF High: {:.0}%", gf_high * 100.0);
    println!("  Surface Pressure: {:.2} bar", surface_pressure);
    println!("  Temperature: {:.1}°C", temperature);
    println!("  (Note: dive-deco library manages gradient factors internally)");
    println!();

    // // Get dive depths from user input
    // println!("\nDive Planning:");
    // let dive_depths = get_depths_input();

    // for &depth in &dive_depths {
    //     plan_dive(depth, gf_high, gf_low);
    //     println!();
    // }

    // Get simulation parameters from user input
    println!("\nDive Simulation:");
    let target_depth = get_float_input("Target depth for simulation (m)", 50.0);
    let bottom_time_minutes = get_float_input("Bottom time for simulation (minutes)", 20.0);

    // Demonstrate a dive simulation
    demonstrate_dive_simulation(target_depth, bottom_time_minutes, gf_high, gf_low);
}

fn demonstrate_dive_simulation(target_depth: f32, bottom_time_minutes: f32, _gf_high: f32, _gf_low: f32) {
    println!("=== Dive Simulation Example (using dive-deco library) ===");
    println!("Simulating dive to {:.0}m for {:.1} minutes...", target_depth, bottom_time_minutes);

    // Create a Bühlmann model
    let mut model = BuehlmannModel::default();
    
    // Standard air composition
    let air = Gas::new(0.21, 0.0);

    // Record the dive
    model.record(
        Depth::from_meters(target_depth),
        Time::from_minutes(bottom_time_minutes),
        &air
    );

    // Check final state
    let final_ceiling = model.ceiling();

    println!("Simulation complete!");
    println!("Final ceiling: {:.1}m", final_ceiling.as_meters());

    if final_ceiling.as_meters() > 0.0 {
        println!("⚠️  Decompression required!");
        
        // Get decompression schedule
        analyze_decompression_schedule(&model);
    } else {
        println!("✅ No decompression required - direct ascent to surface allowed");
    }

    // Compare with NDL - we need to calculate this differently
    // Create a fresh model to calculate NDL
    let fresh_model = BuehlmannModel::default();
    let fresh_ndl = fresh_model.ndl();
    
    println!("\nDive Analysis:");
    println!("Current model NDL: {:.1} minutes", fresh_ndl.as_minutes());
    println!("Actual bottom time: {:.1} minutes", bottom_time_minutes);
    
    if bottom_time_minutes > fresh_ndl.as_minutes() as f32 {
        let overstay = bottom_time_minutes - fresh_ndl.as_minutes() as f32;
        println!("Overstayed NDL by: {:.1} minutes", overstay);
    } else {
        let remaining = fresh_ndl.as_minutes() as f32 - bottom_time_minutes;
        println!("Remaining NDL time: {:.1} minutes", remaining);
    }
}

fn analyze_decompression_schedule(model: &BuehlmannModel) {
    println!("\n=== Decompression Schedule Analysis ===");
    
    // Note: dive-deco library may have different ways to access decompression stops
    // We'll provide what information we can extract from the model
    
    let ceiling = model.ceiling();
    
    println!("Current ceiling: {:.1}m", ceiling.as_meters());
    
    // Additional analysis would require diving deeper into the dive-deco API
    // to extract individual decompression stops, which may not be directly exposed
    println!("\n⚠️  Note: Detailed decompression stop analysis would require");
    println!("   accessing the dive-deco library's internal decompression schedule,");
    println!("   which may not be directly exposed in the public API.");
    println!("   The ceiling provides the essential information for decompression planning.");
    
    // Show gradient factor information
    println!("\nThe dive-deco library uses built-in gradient factor handling.");
    println!("Consult the library documentation for specific GF configuration options.");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_dive_planning() {
        // Test that we can create a model and calculate basic parameters
        let model = BuehlmannModel::default();
        let _air = Gas::new(0.21, 0.0);
        
        // Test NDL calculation
        let ndl = model.ndl();
        assert!(ndl.as_minutes() > 0.0);
        
        // Test ceiling calculation (should be 0 at surface)
        let ceiling = model.ceiling();
        assert_eq!(ceiling.as_meters(), 0.0);
    }
    
    #[test]
    fn test_decompression_dive() {
        // Test a dive that requires decompression
        let mut model = BuehlmannModel::default();
        let air = Gas::new(0.21, 0.0);
        
        // Record a deep, long dive that should require decompression
        model.record(
            Depth::from_meters(50.0),
            Time::from_minutes(20.0),
            &air
        );
        
        let ceiling = model.ceiling();
        
        // This dive should require some decompression
        assert!(ceiling.as_meters() > 0.0);
    }
}
