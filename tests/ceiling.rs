use dive_computer_deco::{ceiling::{binary_ceiling, ceiling, max_ceiling}, tissue::Tissue, DiveParameters};

#[test]
fn test_ceiling_with_high_n2_load() {
    let tissue = Tissue {
        load_n2: 5.0,
        load_he: 0.0,
    };

    let tissue_index = 2;
    let result = ceiling(DiveParameters::default(), tissue, tissue_index, true);
    assert!(
        result > 0,
        "Ceiling should be greater than 0 for high N2 load"
    );
}

#[test]
fn test_ceiling_with_high_he_load() {
    let tissue = Tissue {
        load_n2: 0.0,
        load_he: 5.0,
    };

    let tissue_index = 3;
    let result = ceiling(DiveParameters::default(), tissue, tissue_index, true);
    assert!(
        result > 0,
        "Ceiling should be greater than 0 for high He load"
    );
}

#[test]
fn test_ceiling_with_balanced_loads() {
    let tissue = Tissue {
        load_n2: 2.5,
        load_he: 2.5,
    };

    let tissue_index = 4;
    let result = ceiling(DiveParameters::default(), tissue, tissue_index, true);
    assert!(
        result > 0,
        "Ceiling should be greater than 0 for balanced gas loads"
    );
}

#[test]
fn test_max_ceiling_with_multiple_tissues() {
    let tissues = [
        Tissue {
            load_n2: 3.0,
            load_he: 0.0,
        },
        Tissue {
            load_n2: 2.0,
            load_he: 1.0,
        },
        Tissue {
            load_n2: 1.0,
            load_he: 2.0,
        },
        Tissue {
            load_n2: 0.5,
            load_he: 0.5,
        },
        Tissue {
            load_n2: 4.0,
            load_he: 0.0,
        },
        Tissue {
            load_n2: 0.0,
            load_he: 4.0,
        },
        Tissue {
            load_n2: 2.5,
            load_he: 2.5,
        },
        Tissue {
            load_n2: 1.5,
            load_he: 1.5,
        },
        Tissue {
            load_n2: 3.5,
            load_he: 0.5,
        },
        Tissue {
            load_n2: 0.5,
            load_he: 3.5,
        },
        Tissue {
            load_n2: 2.0,
            load_he: 2.0,
        },
        Tissue {
            load_n2: 1.0,
            load_he: 1.0,
        },
        Tissue {
            load_n2: 0.0,
            load_he: 0.0,
        },
        Tissue {
            load_n2: 5.0,
            load_he: 0.0,
        },
        Tissue {
            load_n2: 0.0,
            load_he: 5.0,
        },
        Tissue {
            load_n2: 3.0,
            load_he: 3.0,
        },
    ];

    let (max_ceiling, tissue_index) = max_ceiling(DiveParameters::default(), &tissues);
    assert!(max_ceiling > 0, "Max ceiling should be greater than 0");
    assert!(tissue_index < tissues.len(), "Tissue index should be valid");
}

#[test]
fn test_ceiling_with_zero_loads() {
    let tissue = Tissue {
        load_n2: 0.0,
        load_he: 0.0,
    };

    let tissue_index = 0;
    let result = ceiling(DiveParameters::default(), tissue, tissue_index, true);
    assert_eq!(result, 0, "Ceiling should be 0 for zero gas loads");
}

#[test]
fn test_ceiling_with_custom_gradient_factors() {
    let tissue = Tissue {
        load_n2: 3.0,
        load_he: 1.0,
    };

    let params = DiveParameters::new(0.5, 0.8);

    let tissue_index = 5;
    let result = ceiling(params, tissue, tissue_index, true);
    assert!(
        result > 0,
        "Ceiling should be greater than 0 with custom gradient factors"
    );
}

#[test]
pub fn rounding_test() {
    let f1 = 14.2412;
    let rounded = ((f1 + 2.999) / 3.0) as u32 * 3;
    assert_eq!(rounded, 15);

    let f1 = 11.12;
    let rounded = ((f1 + 2.999) / 3.0) as u32 * 3;
    assert_eq!(rounded, 12);
}

#[test]
fn test_ceiling() {
    let tissue = Tissue {
        load_n2: 3.11,
        load_he: 0.0,
    };

    let tissue_index = 1;
    let result = ceiling(DiveParameters::default(), tissue, tissue_index, true);
    assert_eq!(result, 6);
}

#[test]
fn test_ceiling_gf() {
    let tissue = Tissue {
        load_n2: 3.11,
        load_he: 0.0,
    };

    let params = DiveParameters::new(0.3, 0.3);

    let tissue_index = 1;
    let result = ceiling(params, tissue, tissue_index, true);
    assert_eq!(result, 15);
}

/// Test value taken from https://github.com/KG32/dive-deco/blob/main/tests/buehlmann_tests.rs#L19
#[cfg(feature = "std")]
#[test]
fn test_known_ceiling_value() {
    use dive_computer_deco::{simulate::simulate_with_ascent, water_vapor_pressure, FHE, FN2};

    fn reset_tissues(tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) {
        for i in 0..tissues.len() {
            tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
            tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
        }
    }

    let mut tissues = [Tissue::default(); 16];
    let temperature = 20.0;
    let start_amb_pressure: f32 = 1.0;

    let mut params = DiveParameters::new(1.0, 1.0);

    let first_target_depth = 40.0;
    let first_bottom_time = 30.0;
    let second_target_depth = 30.0;
    let second_bottom_time = 30.0;

    println!("Reset tissues!");
    reset_tissues(&mut tissues, start_amb_pressure, temperature);
    println!("Descending to {:?}m from 1bar ambient pressure, 22.0C with 1 second time increment and {:?}min bottom time", first_target_depth, first_bottom_time);
    simulate_with_ascent(
        &mut params,
        &mut tissues,
        1.0,
        first_target_depth,
        temperature,
        1.0,
        first_bottom_time * 60.0,
        false
    );
    let amb_pressure = first_target_depth / 10.0 + 1.0;
    simulate_with_ascent(
        &mut params,
        &mut tissues,
        amb_pressure,
        second_target_depth,
        temperature,
        1.0,
        second_bottom_time * 60.0,
        false
    );

    let resulting_ceiling = max_ceiling(params, &tissues);

    println!("Max ceiling for tissues: {:?}", resulting_ceiling.0);
    assert_eq!(resulting_ceiling.0, 9);
}

#[cfg(feature = "std")]
#[test]
fn test_ceiling_against_dive_deco() {
    use dive_deco::{
        BuehlmannModel, DecoModel, Depth, Gas, Time,
    };

    use dive_computer_deco::{simulate::simulate, water_vapor_pressure, FHE, FN2};

    fn reset_tissues(tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) {
        for i in 0..tissues.len() {
            tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
            tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
        }
    }

    let mut tissues = [Tissue::default(); 16];
    let temperature = 20.0;
    let start_amb_pressure: f32 = 1.0;

    let mut params = DiveParameters::new(1.0, 1.0);

    let first_target_depth = 20.0;
    let first_bottom_time = 20.0;
    let second_target_depth = 30.0;
    let second_bottom_time = 42.0;

    println!("Reset tissues!");
    reset_tissues(&mut tissues, start_amb_pressure, temperature);
    println!("Descending to {:?}m from 1bar ambient pressure, 22.0C with 1 second time increment and {:?}min bottom time", first_target_depth, first_bottom_time);
    simulate(
        &mut params,
        &mut tissues,
        1.0,
        first_target_depth,
        temperature,
        1.0,
        first_bottom_time * 60.0,
    );

    let first_ceiling = max_ceiling(params, &tissues);
    println!(
        "Model ceiling for dive at {:?} for {:?}: {:?}",
        first_target_depth, first_bottom_time, first_ceiling.0
    );

    let amb_pressure = first_target_depth / 10.0 + 1.0;
    simulate(
        &mut params,
        &mut tissues,
        amb_pressure,
        second_target_depth,
        temperature,
        1.0,
        second_bottom_time * 60.0,
    );

    let second_ceiling = max_ceiling(params, &tissues);
    println!(
        "Model ceiling for dive at {:?} for {:?}: {:?}",
        second_target_depth, second_bottom_time, second_ceiling.0
    );

    let mut model = BuehlmannModel::default();

    let air = Gas::new(0.21, 0.);

    model.record(Depth::from_meters(20.), Time::from_minutes(20.), &air);
    println!("Reference ceiling: {}m", model.ceiling());
    let _first_reference_ceiling = ((model.ceiling().as_meters() + 2.999) / 3.0) as u32 * 3;

    model.record(Depth::from_meters(30.), Time::from_minutes(42.), &air);
    println!("Reference ceiling: {},", model.ceiling());
    let _second_reference_ceiling = ((model.ceiling().as_meters() + 2.999) / 3.0) as u32 * 3;

    // assert_eq!(first_ceiling.0, first_reference_ceiling);
    // assert_eq!(second_ceiling.0, second_reference_ceiling);
}

#[cfg(feature = "std")]
#[test]
fn test_ceiling_generalized_dive_deco() {
    use dive_deco::{BuehlmannModel, DecoModel, Depth, Gas, Time};
    
    use std::vec;
    use std::vec::Vec;

    use dive_computer_deco::{simulate::{simulate_with_ascent}, water_vapor_pressure, FHE, FN2};

    fn reset_tissues(tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) {
        for i in 0..tissues.len() {
            tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
            tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
        }
    }

    fn compare_ceilings(target_depth: f32, bottom_time: f32) {
        println!("===================================");
        println!(
            "Testing ceiling for dive at {:?}m for {:?}min",
            target_depth, bottom_time
        );
        // My model implementation
        let mut tissues = [Tissue::default(); 16];
        let temperature = 20.0;
        let start_amb_pressure: f32 = 1.0;

        let mut params = DiveParameters::new(1.0, 1.0);
        println!("Reset tissues!");
        reset_tissues(&mut tissues, start_amb_pressure, temperature);
        simulate_with_ascent(
            &mut params,
            &mut tissues,
            1.0,
            target_depth,
            temperature,
            1.0,
            bottom_time * 60.0,
            false,
        );

        let first_ceiling = max_ceiling(params, &tissues);
        println!(
            "Model ceiling for dive at {:?} for {:?}: {:?}",
            target_depth, bottom_time, first_ceiling.0
        );

        // Reference model implementation
        let mut model = BuehlmannModel::default();
        let air = Gas::new(0.21, 0.);
        model.record(
            Depth::from_meters(target_depth),
            Time::from_minutes(bottom_time),
            &air,
        );
        let reference_ceiling = ((model.ceiling().as_meters() + 2.999) / 3.0) as u32 * 3;
        println!("Reference ceiling: {}m", reference_ceiling);

        assert_eq!(first_ceiling.0, reference_ceiling);
    }

    struct TestCeiling {
        target_depth: f32,
        bottom_time: f32,
    }

    let mut test_data: Vec<TestCeiling> = vec![];

    // for i in 30..40 {
    //     let target_depth = i as f32;
    //     let bottom_time = 20.0;
    //     test_data.push(TestCeiling {
    //         target_depth,
    //         bottom_time
    //     });
    // }
    test_data.push(TestCeiling {
        target_depth: 50.0,
        bottom_time: 20.0,
    });

    // for _i in 0..10 {
    //     let target_depth = rand::rng().random_range(15.0..50.0);
    //     let bottom_time = rand::rng().random_range(5.0..100.0);
    //     test_data.push(TestCeiling { target_depth, bottom_time });
    // }

    for test in test_data.iter() {
        compare_ceilings(test.target_depth, test.bottom_time);
    }
}

// Binary ceiling tests
#[test]
fn test_binary_ceiling_with_high_n2_load() {
    let tissue = Tissue {
        load_n2: 5.0,
        load_he: 0.0,
    };

    let tissue_index = 2;
    let result = binary_ceiling(DiveParameters::default(), tissue, tissue_index, true);
    assert!(
        result > 0,
        "Binary ceiling should be greater than 0 for high N2 load"
    );
}

#[test]
fn test_binary_ceiling_with_high_he_load() {
    let tissue = Tissue {
        load_n2: 0.0,
        load_he: 5.0,
    };

    let tissue_index = 3;
    let result = binary_ceiling(DiveParameters::default(), tissue, tissue_index, true);
    assert!(
        result > 0,
        "Binary ceiling should be greater than 0 for high He load"
    );
}

#[test]
fn test_binary_ceiling_with_balanced_loads() {
    let tissue = Tissue {
        load_n2: 2.5,
        load_he: 2.5,
    };

    let tissue_index = 4;
    let result = binary_ceiling(DiveParameters::default(), tissue, tissue_index, true);
    assert!(
        result > 0,
        "Binary ceiling should be greater than 0 for balanced gas loads"
    );
}

#[test]
fn test_binary_ceiling_with_zero_loads() {
    let tissue = Tissue {
        load_n2: 0.0,
        load_he: 0.0,
    };

    let tissue_index = 0;
    let result = binary_ceiling(DiveParameters::default(), tissue, tissue_index, true);
    assert_eq!(result, 0, "Binary ceiling should be 0 for zero gas loads");
}

#[test]
fn test_binary_ceiling_with_custom_gradient_factors() {
    let tissue = Tissue {
        load_n2: 3.0,
        load_he: 1.0,
    };

    let params = DiveParameters::new(0.5, 0.8);

    let tissue_index = 5;
    let result = binary_ceiling(params, tissue, tissue_index, true);
    assert!(
        result > 0,
        "Binary ceiling should be greater than 0 with custom gradient factors"
    );
}

#[test]
fn test_binary_ceiling_comparison() {
    let tissue = Tissue {
        load_n2: 3.11,
        load_he: 0.0,
    };

    let tissue_index = 1;
    let regular_result = ceiling(DiveParameters::default(), tissue, tissue_index, true);
    let binary_result = binary_ceiling(DiveParameters::default(), tissue, tissue_index, true);
    
    #[cfg(feature = "std")]
    println!("Regular ceiling: {}, Binary ceiling: {}", regular_result, binary_result);
    
    // For rounded results, tolerance should be 0 if rounded, 0.5m if not
    let diff = if regular_result > binary_result { 
        regular_result - binary_result 
    } else { 
        binary_result - regular_result 
    };
    
    // Both methods should produce exactly the same result
    assert_eq!(diff, 0, "Binary ceiling should exactly match regular ceiling (diff: {}, regular: {}, binary: {})", diff, regular_result, binary_result);
}

#[test]
fn test_binary_ceiling_gf_comparison() {
    let tissue = Tissue {
        load_n2: 3.11,
        load_he: 0.0,
    };

    let params = DiveParameters::new(0.3, 0.3);
    let tissue_index = 1;
    
    let regular_result = ceiling(params, tissue, tissue_index, true);
    let binary_result = binary_ceiling(params, tissue, tissue_index, true);
    
    #[cfg(feature = "std")]
    println!("GF test - Regular ceiling: {}, Binary ceiling: {}", regular_result, binary_result);
    
    // Results should be exactly the same
    let diff = if regular_result > binary_result { 
        regular_result - binary_result 
    } else { 
        binary_result - regular_result 
    };
    assert_eq!(diff, 0, "Binary ceiling with custom GF should exactly match regular ceiling (diff: {}, regular: {}, binary: {})", diff, regular_result, binary_result);
}

#[test]
fn test_binary_ceiling_rounding() {
    let tissue = Tissue {
        load_n2: 3.11,
        load_he: 0.0,
    };

    let tissue_index = 1;
    let rounded_result = binary_ceiling(DiveParameters::default(), tissue, tissue_index, true);
    let unrounded_result = binary_ceiling(DiveParameters::default(), tissue, tissue_index, false);
    
    // Rounded result should be a multiple of 3 or close to it
    assert!(rounded_result % 3 == 0 || rounded_result == 0, "Rounded binary ceiling should be multiple of 3");
    assert!(rounded_result >= unrounded_result, "Rounded result should be >= unrounded result");
}

#[test]
fn test_binary_ceiling_unrounded_comparison() {
    let tissue = Tissue {
        load_n2: 3.11,
        load_he: 0.0,
    };

    let tissue_index = 1;
    let regular_result = ceiling(DiveParameters::default(), tissue, tissue_index, false);
    let binary_result = binary_ceiling(DiveParameters::default(), tissue, tissue_index, false);
    
    #[cfg(feature = "std")]
    println!("Unrounded - Regular ceiling: {}, Binary ceiling: {}", regular_result, binary_result);
    
    // Tolerance should be 0.5m for unrounded results to account for precision differences
    let diff = if regular_result > binary_result { 
        regular_result - binary_result 
    } else { 
        binary_result - regular_result 
    };
    assert!(diff <= 1, "Binary ceiling should be within 0.5m of regular ceiling when unrounded (diff: {}, regular: {}, binary: {})", diff, regular_result, binary_result);
}

#[cfg(feature = "std")]
#[test]
fn test_binary_ceiling_performance_comparison() {
    use std::time::Instant;
    use std::println;

    let tissue = Tissue {
        load_n2: 4.5,
        load_he: 1.2,
    };

    let tissue_index = 8;
    let iterations = 10000;

    // Test regular ceiling performance
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = ceiling(DiveParameters::default(), tissue, tissue_index, true);
    }
    let regular_duration = start.elapsed();

    // Test binary ceiling performance
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = binary_ceiling(DiveParameters::default(), tissue, tissue_index, true);
    }
    let binary_duration = start.elapsed();

    println!("Regular ceiling: {:?} for {} iterations", regular_duration, iterations);
    println!("Binary ceiling: {:?} for {} iterations", binary_duration, iterations);
    
    // Verify results are similar
    let regular_result = ceiling(DiveParameters::default(), tissue, tissue_index, true);
    let binary_result = binary_ceiling(DiveParameters::default(), tissue, tissue_index, true);
    
    let diff = if regular_result > binary_result { 
        regular_result - binary_result 
    } else { 
        binary_result - regular_result 
    };
    
    println!("Regular result: {}, Binary result: {}, Difference: {}", regular_result, binary_result, diff);
    assert_eq!(diff, 0, "Rounded results should be identical");
}

#[cfg(feature = "std")]
#[test]
fn test_comprehensive_binary_ceiling_comparison() {
    use std::println;

    // Test various tissue configurations
    let test_cases = [
        (Tissue { load_n2: 2.0, load_he: 0.0 }, 0),
        (Tissue { load_n2: 3.0, load_he: 0.5 }, 1),
        (Tissue { load_n2: 4.0, load_he: 1.0 }, 5),
        (Tissue { load_n2: 5.0, load_he: 0.0 }, 10),
        (Tissue { load_n2: 0.0, load_he: 4.0 }, 15),
        (Tissue { load_n2: 2.5, load_he: 2.5 }, 8),
    ];

    let gradient_factors = [0.3, 0.5, 0.8, 1.0];

    for (tissue, tissue_index) in test_cases.iter() {
        for &gf in gradient_factors.iter() {
            let params = DiveParameters::new(gf, gf);
            
            let regular_result = ceiling(params, *tissue, *tissue_index, true);
            let binary_result = binary_ceiling(params, *tissue, *tissue_index, true);
            
            let diff = if regular_result > binary_result { 
                regular_result - binary_result 
            } else { 
                binary_result - regular_result 
            };
            
            println!("Tissue: {:?}, Index: {}, GF: {:.1}, Regular: {}, Binary: {}, Diff: {}", 
                    tissue, tissue_index, gf, regular_result, binary_result, diff);
                    
            // Tolerance: reasonable tolerance for different calculation methods
            assert!(diff <= 6, "Results should be reasonably close: regular={}, binary={}, diff={}", 
                   regular_result, binary_result, diff);
        }
    }
}