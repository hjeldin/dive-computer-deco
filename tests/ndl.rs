use dive_computer_deco::ndl::binary_ndl;
use dive_computer_deco::ndl::ndl;
use dive_computer_deco::simulate::simulate;
use dive_computer_deco::{water_vapor_pressure, DiveParameters, FHE, FN2};
use dive_computer_deco::tissue::Tissue;
// Binary NDL tests
#[test]
fn test_binary_ndl_vs_regular() {
    fn reset_tissues(tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) {
        for i in 0..tissues.len() {
            tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
            tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
        }
    }

    let mut tissues_regular = [Tissue::default(); 16];
    let mut tissues_binary = [Tissue::default(); 16];
    let temperature = 20.0;
    let start_amb_pressure = 1.0;
    let target_depth = 30.0;

    let params = DiveParameters::new(1.0, 1.0);

    // Reset both tissue arrays to same initial state
    reset_tissues(&mut tissues_regular, start_amb_pressure, temperature);
    reset_tissues(&mut tissues_binary, start_amb_pressure, temperature);

    // Simulate descent for both (this modifies tissues but consistently)
    simulate(
        &mut DiveParameters::new(1.0, 1.0),
        &mut tissues_regular,
        1.0,
        target_depth,
        temperature,
        1.0,
        0.0,
    );
    simulate(
        &mut DiveParameters::new(1.0, 1.0),
        &mut tissues_binary,
        1.0,
        target_depth,
        temperature,
        1.0,
        0.0,
    );

    let amb_pressure = target_depth / 10.0 + 1.0;

    // Calculate NDL with both methods
    let regular_ndl = ndl(params, &mut tissues_regular, amb_pressure, temperature);
    let binary_ndl = binary_ndl(params, &mut tissues_binary, amb_pressure, temperature);

    #[cfg(feature = "std")]
    println!("Regular NDL: {}, Binary NDL: {}", regular_ndl, binary_ndl);

    // Results should be exactly the same
    let diff = if regular_ndl > binary_ndl {
        regular_ndl - binary_ndl
    } else {
        binary_ndl - regular_ndl
    };
    assert_eq!(
        diff, 0.0,
        "Binary NDL should exactly match regular NDL (diff: {}, regular: {}, binary: {})",
        diff, regular_ndl, binary_ndl
    );
}

#[test]
fn test_binary_ndl_shallow_depth() {
    fn reset_tissues(tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) {
        for i in 0..tissues.len() {
            tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
            tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
        }
    }

    let mut tissues = [Tissue::default(); 16];
    let temperature = 20.0;
    let start_amb_pressure = 1.0;
    let target_depth = 15.0; // Shallow depth should have long NDL

    let params = DiveParameters::new(1.0, 1.0);

    reset_tissues(&mut tissues, start_amb_pressure, temperature);
    simulate(
        &mut DiveParameters::new(1.0, 1.0),
        &mut tissues,
        1.0,
        target_depth,
        temperature,
        1.0,
        0.0,
    );

    let amb_pressure = target_depth / 10.0 + 1.0;
    let result = binary_ndl(params, &mut tissues, amb_pressure, temperature);

    assert!(
        result > 50.0,
        "Binary NDL for shallow depth should be > 50 minutes, got: {}",
        result
    );
}

#[test]
fn test_binary_ndl_deep_depth() {
    fn reset_tissues(tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) {
        for i in 0..tissues.len() {
            tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
            tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
        }
    }

    let mut tissues = [Tissue::default(); 16];
    let temperature = 20.0;
    let start_amb_pressure = 1.0;
    let target_depth = 50.0; // Deep depth should have short NDL

    let params = DiveParameters::new(1.0, 1.0);

    reset_tissues(&mut tissues, start_amb_pressure, temperature);
    simulate(
        &mut DiveParameters::new(1.0, 1.0),
        &mut tissues,
        1.0,
        target_depth,
        temperature,
        1.0,
        0.0,
    );

    let amb_pressure = target_depth / 10.0 + 1.0;
    let result = binary_ndl(params, &mut tissues, amb_pressure, temperature);

    assert!(
        result < 20.0,
        "Binary NDL for deep depth should be < 20 minutes, got: {}",
        result
    );
}

#[test]
fn test_binary_ndl_with_custom_gradient_factors() {
    fn reset_tissues(tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) {
        for i in 0..tissues.len() {
            tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
            tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
        }
    }

    let mut tissues_conservative = [Tissue::default(); 16];
    let mut tissues_aggressive = [Tissue::default(); 16];
    let temperature = 20.0;
    let start_amb_pressure = 1.0;
    let target_depth = 30.0;

    let conservative_params = DiveParameters::new(0.3, 0.3); // Very conservative
    let aggressive_params = DiveParameters::new(1.0, 1.0);   // Less conservative

    // Test conservative gradient factors
    reset_tissues(&mut tissues_conservative, start_amb_pressure, temperature);
    simulate(
        &mut DiveParameters::new(1.0, 1.0),
        &mut tissues_conservative,
        1.0,
        target_depth,
        temperature,
        1.0,
        0.0,
    );

    // Test aggressive gradient factors  
    reset_tissues(&mut tissues_aggressive, start_amb_pressure, temperature);
    simulate(
        &mut DiveParameters::new(1.0, 1.0),
        &mut tissues_aggressive,
        1.0,
        target_depth,
        temperature,
        1.0,
        0.0,
    );

    let amb_pressure = target_depth / 10.0 + 1.0;

    let conservative_ndl = binary_ndl(conservative_params, &mut tissues_conservative, amb_pressure, temperature);
    let aggressive_ndl = binary_ndl(aggressive_params, &mut tissues_aggressive, amb_pressure, temperature);

    #[cfg(feature = "std")]
    println!("Conservative NDL (GF 30/30): {}, Aggressive NDL (GF 100/100): {}", conservative_ndl, aggressive_ndl);

    assert!(conservative_ndl <= aggressive_ndl, "Conservative gradient factors should result in shorter or equal NDL");
}

#[cfg(feature = "std")]
#[test]
fn test_binary_ndl_performance_comparison() {    use std::time::Instant;

    fn reset_tissues(tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) {
        for i in 0..tissues.len() {
            tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
            tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
        }
    }

    let temperature = 20.0;
    let start_amb_pressure = 1.0;
    let target_depth = 30.0;
    let params = DiveParameters::new(1.0, 1.0);
    let iterations = 10;

    // Test regular NDL performance
    let start = Instant::now();
    for _ in 0..iterations {
        let mut tissues = [Tissue::default(); 16];
        reset_tissues(&mut tissues, start_amb_pressure, temperature);
        simulate(
            &mut DiveParameters::new(1.0, 1.0),
            &mut tissues,
            1.0,
            target_depth,
            temperature,
            1.0,
            0.0,
        );
        let amb_pressure = target_depth / 10.0 + 1.0;
        let _ = ndl(params, &mut tissues, amb_pressure, temperature);
    }
    let regular_duration = start.elapsed();

    // Test binary NDL performance
    let start = Instant::now();
    for _ in 0..iterations {
        let mut tissues = [Tissue::default(); 16];
        reset_tissues(&mut tissues, start_amb_pressure, temperature);
        simulate(
            &mut DiveParameters::new(1.0, 1.0),
            &mut tissues,
            1.0,
            target_depth,
            temperature,
            1.0,
            0.0,
        );
        let amb_pressure = target_depth / 10.0 + 1.0;
        let _ = binary_ndl(params, &mut tissues, amb_pressure, temperature);
    }
    let binary_duration = start.elapsed();

    println!("Regular NDL: {:?} for {} iterations", regular_duration, iterations);
    println!("Binary NDL: {:?} for {} iterations", binary_duration, iterations);
    
    // Verify results are similar
    let mut tissues_regular = [Tissue::default(); 16];
    let mut tissues_binary = [Tissue::default(); 16];
    
    reset_tissues(&mut tissues_regular, start_amb_pressure, temperature);
    reset_tissues(&mut tissues_binary, start_amb_pressure, temperature);
    
    simulate(&mut DiveParameters::new(1.0, 1.0), &mut tissues_regular, 1.0, target_depth, temperature, 1.0, 0.0);
    simulate(&mut DiveParameters::new(1.0, 1.0), &mut tissues_binary, 1.0, target_depth, temperature, 1.0, 0.0);
    
    let amb_pressure = target_depth / 10.0 + 1.0;
    let regular_result = ndl(params, &mut tissues_regular, amb_pressure, temperature);
    let binary_result = binary_ndl(params, &mut tissues_binary, amb_pressure, temperature);
    
    let diff = if regular_result > binary_result { 
        regular_result - binary_result 
    } else { 
        binary_result - regular_result 
    };
    
    println!("Regular result: {}, Binary result: {}, Difference: {}", regular_result, binary_result, diff);
    assert_eq!(diff, 0.0, "Results should match exactly");
}

#[cfg(feature = "std")]
#[test]
fn test_comprehensive_binary_ndl_comparison() {
    fn reset_tissues(tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) {
        for i in 0..tissues.len() {
            tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
            tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
        }
    }

    let test_depths = [15.0, 21.0, 27.0, 33.0, 39.0, 45.0];
    let gradient_factors = [0.3, 0.5, 0.8, 1.0];
    let temperature = 20.0;
    let start_amb_pressure = 1.0;

    for &depth in test_depths.iter() {
        for &gf in gradient_factors.iter() {
            let params = DiveParameters::new(gf, gf);
            
            let mut tissues_regular = [Tissue::default(); 16];
            let mut tissues_binary = [Tissue::default(); 16];
            
            reset_tissues(&mut tissues_regular, start_amb_pressure, temperature);
            reset_tissues(&mut tissues_binary, start_amb_pressure, temperature);
            
            simulate(&mut DiveParameters::new(1.0, 1.0), &mut tissues_regular, 1.0, depth, temperature, 1.0, 0.0);
            simulate(&mut DiveParameters::new(1.0, 1.0), &mut tissues_binary, 1.0, depth, temperature, 1.0, 0.0);
            
            let amb_pressure = depth / 10.0 + 1.0;
            
            let regular_ndl = ndl(params, &mut tissues_regular, amb_pressure, temperature);
            let binary_ndl = binary_ndl(params, &mut tissues_binary, amb_pressure, temperature);
            
            let diff = if regular_ndl > binary_ndl { 
                regular_ndl - binary_ndl 
            } else { 
                binary_ndl - regular_ndl 
            };
            
            println!("Depth: {}m, GF: {:.1}, Regular: {}min, Binary: {}min, Diff: {}min", 
                    depth, gf, regular_ndl, binary_ndl, diff);
                    
            assert_eq!(diff, 0.0, "NDL methods should produce exactly the same result: regular={}, binary={}, diff={}", 
                   regular_ndl, binary_ndl, diff);
        }
    }
}
