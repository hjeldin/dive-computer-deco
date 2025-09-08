
#[cfg(feature = "std")]
#[test]
fn test_dive_deco() {
    use dive_deco::{BuhlmannConfig, BuhlmannModel, DecoModel, DecoStage, Depth, Gas, Time};

    let config = BuhlmannConfig::new().with_gradient_factors(80, 80);
    let mut model = BuhlmannModel::default();
    model.update_config(config);

    // bottom gas
    let air = Gas::air();
    let available_gas_mixes = vec![
        air,
    ];

    let bottom_depth = Depth::from_meters(50.);
    let bottom_time = Time::from_seconds(20 * 60); // 20 min

    // descent to 50m using air
    model.record_travel(bottom_depth, Time::from_seconds(2.5 * 60.), &air);

    // 20 min bottom time
    model.record(bottom_depth, bottom_time, &air);

    // calculate deco runtime providing available gasses
    let deco_runtime = model.deco(available_gas_mixes);

    println!("{:#?}", config);
    println!("{:#?}", deco_runtime);
    let mut deco_time = 0.;
    match deco_runtime {
        Ok(runtime) => {
            // println!("Decompression runtime: {:?}", runtime);
            let mut current_time = model.dive_state().time.as_seconds();
             println!("Depth (m) | Duration (secs) | Duration (min) | Start Time (min) | End Time (min)");
            println!("----------|----------------|----------------|------------------|----------------");
            runtime.deco_stages.iter().for_each(|stage: &DecoStage| {
                if stage.stage_type == dive_deco::DecoStageType::DecoStop {
                    println!("   {:4.1}   |     {:6.1}     |      {:6.1}      |      {:6.1}      |      {:6.1}", stage.start_depth.as_meters(), stage.duration.as_seconds(), stage.duration.as_minutes(), current_time / 60.0, current_time / 60.0 + stage.duration.as_minutes());
                    deco_time += stage.duration.as_minutes();
                }
                current_time += stage.duration.as_seconds();
            });
        }
        Err(e) => {
            println!("Error calculating deco runtime: {:?}", e);
        }
    }

    println!("Total deco time: {:.1} minutes", deco_time);
}

#[cfg(feature = "std")]
#[test]
fn test_binary_ceiling_vs_regular() {
    use dive_computer_deco::{DiveParameters, water_vapor_pressure, FN2, FHE};
    use dive_computer_deco::tissue::{Tissue, calculate_tissue};
    use dive_computer_deco::ceiling::{ceiling, binary_ceiling};

    println!("=== BINARY CEILING vs REGULAR CEILING COMPARISON ===");

    let temperature = 20.0;
    let start_amb_pressure = 1.0;
    let target_depth = 50.0;
    let bottom_time_minutes = 20.0;
    
    // Prepare tissues after bottom time
    let amb_pressure_at_depth = target_depth / 10.0 + 1.0;
    let time_step_minutes = 1.0 / 60.0; // 1 second steps
    let num_steps = (bottom_time_minutes * 60.0) as i32;
    
    let mut tissues = [Tissue::default(); 16];
    for i in 0..16 {
        tissues[i].load_n2 = (start_amb_pressure - water_vapor_pressure(temperature)) * FN2;
        tissues[i].load_he = (start_amb_pressure - water_vapor_pressure(temperature)) * FHE;
        
        // Simulate bottom time
        for _step in 0..num_steps {
            tissues[i] = calculate_tissue(tissues[i], i, amb_pressure_at_depth, temperature, time_step_minutes);
        }
    }
    
    println!("\n--- Ceiling Calculation Comparison ---");
    let dive_params = DiveParameters::new(1.0, 1.0); // GF 100/100
    
    // Check each tissue's ceiling calculation
    for i in 0..16 {
        let regular_ceiling = ceiling(dive_params, tissues[i], i, true);
        let binary_ceiling_result = binary_ceiling(dive_params, tissues[i], i, true);
        
        if regular_ceiling > 0 || binary_ceiling_result > 0 {
            let diff = if regular_ceiling > binary_ceiling_result { 
                regular_ceiling - binary_ceiling_result 
            } else { 
                binary_ceiling_result - regular_ceiling 
            };
            
            println!("Tissue {:2}: N2={:.6} bar, Regular={:2}m, Binary={:2}m, Diff={:2}m", 
                i + 1, tissues[i].load_n2, regular_ceiling, binary_ceiling_result, diff);
        }
    }
    
    // Test with different gradient factors too
    println!("\n--- Testing with GF 30/30 ---");
    let conservative_params = DiveParameters::new(0.3, 0.3);
    
    for i in 0..16 {
        let regular_ceiling = ceiling(conservative_params, tissues[i], i, true);
        let binary_ceiling_result = binary_ceiling(conservative_params, tissues[i], i, true);
        
        if regular_ceiling > 0 || binary_ceiling_result > 0 {
            let diff = if regular_ceiling > binary_ceiling_result { 
                regular_ceiling - binary_ceiling_result 
            } else { 
                binary_ceiling_result - regular_ceiling 
            };
            
            println!("Tissue {:2}: Regular={:2}m, Binary={:2}m, Diff={:2}m", 
                i + 1, regular_ceiling, binary_ceiling_result, diff);
        }
    }
}