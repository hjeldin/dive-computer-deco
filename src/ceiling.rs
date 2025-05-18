use crate::tissue::Tissue;
use crate::zh16c::ZhL16cGf;
use crate::DiveParameters;
#[cfg(feature = "std")]
use std::println;

#[inline(never)]
pub fn ceiling(dive_parameters: DiveParameters, tissue: Tissue, tissue_index: usize) -> u32 {
    let pn2 = tissue.load_n2;
    let phe = tissue.load_he;
    let an2: f32 = ZhL16cGf::N2_A[tissue_index];
    let bn2: f32 = ZhL16cGf::N2_B[tissue_index];

    let ahe: f32 = ZhL16cGf::HE_A[tissue_index];
    let bhe: f32 = ZhL16cGf::HE_B[tissue_index];

    let p_total = pn2 + phe;
    let a = ((an2 * pn2) + (ahe * phe)) / (p_total);
    let b = ((bn2 * pn2) + (bhe * phe)) / (p_total);

    // let r = ((p_total) - a * dive_parameters.gf_high) * (b / (dive_parameters.gf_high - (dive_parameters.gf_high * b) + b));
    // let mut result_bar = (p_total) - a * dive_parameters.gf_high;
    // result_bar /= (dive_parameters.gf_high / b) + 1.0 - dive_parameters.gf_high;
    let result_bar =
        (b * p_total - dive_parameters.gf_low * a * b) / ((1.0 - b) * dive_parameters.gf_low + b);

    // the result is in bars, we need to convert it to meters
    let result_meters = (result_bar - 1.0) * 10.0;

    // round down to multiples of 3
    let ceiling = ((result_meters + 2.999) / 3.0) as u32 * 3;
    // let rounded_ceiling = (ceiling * 3.0) as f32;
    #[cfg(feature = "std")]
    println!(
        "Tissue: {:?} \t Ceil (nr): {:.5} \t Ceil: {:.5}",
        tissue_index + 1,
        result_meters,
        ceiling
    );
    ceiling
}

#[inline(never)]
pub fn max_ceiling(dive_parameters: DiveParameters, tissues: &[Tissue; 16]) -> (u32, usize) {
    let mut max_ceiling = 0;
    let mut tissue_index = 0;
    for i in 0..16 {
        let tentative_max_ceiling = ceiling(dive_parameters, tissues[i], i);
        if tentative_max_ceiling > max_ceiling {
            max_ceiling = tentative_max_ceiling;
            tissue_index = i;
        }
    }
    (max_ceiling, tissue_index)
}

#[test]
fn test_ceiling_with_high_n2_load() {
    let tissue = Tissue {
        load_n2: 5.0,
        load_he: 0.0,
    };

    let tissue_index = 2;
    let result = ceiling(DiveParameters::default(), tissue, tissue_index);
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
    let result = ceiling(DiveParameters::default(), tissue, tissue_index);
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
    let result = ceiling(DiveParameters::default(), tissue, tissue_index);
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
    let result = ceiling(DiveParameters::default(), tissue, tissue_index);
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
    let result = ceiling(params, tissue, tissue_index);
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
    let result = ceiling(DiveParameters::default(), tissue, tissue_index);
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
    let result = ceiling(params, tissue, tissue_index);
    assert_eq!(result, 15);
}

/// Test value taken from https://github.com/KG32/dive-deco/blob/main/tests/buehlmann_tests.rs#L19
#[cfg(feature = "std")]
#[test]
fn test_known_ceiling_value() {
    use crate::{simulate::simulate, water_vapor_pressure, FHE, FN2};

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
    simulate(
        &mut params,
        &mut tissues,
        1.0,
        first_target_depth,
        temperature,
        1.0,
        first_bottom_time * 60.0,
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

    let resulting_ceiling = max_ceiling(params, &tissues);

    println!("Max ceiling for tissues: {:?}", resulting_ceiling.0);
    assert_eq!(resulting_ceiling.0, 9);
}

#[cfg(feature = "std")]
#[test]
fn test_ceiling_against_dive_deco() {
    use dive_deco::{
        BuehlmannConfig, BuehlmannModel, CeilingType, DecoModel, DecoRuntime, DecoStage,
        DecoStageType, Depth, Gas, Time,
    };

    use crate::{simulate::simulate, water_vapor_pressure, FHE, FN2};

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
    let first_reference_ceiling = ((model.ceiling().as_meters() + 2.999) / 3.0) as u32 * 3;

    model.record(Depth::from_meters(30.), Time::from_minutes(42.), &air);
    println!("Reference ceiling: {},", model.ceiling());
    let second_reference_ceiling = ((model.ceiling().as_meters() + 2.999) / 3.0) as u32 * 3;

    // assert_eq!(first_ceiling.0, first_reference_ceiling);
    // assert_eq!(second_ceiling.0, second_reference_ceiling);
}

#[cfg(feature = "std")]
#[test]
fn test_ceiling_generalized_dive_deco() {
    use dive_deco::{BuehlmannModel, DecoModel, Depth, Gas, Time};
    use rand::Rng;
    use std::vec;
    use std::vec::Vec;

    use crate::{simulate::simulate, water_vapor_pressure, FHE, FN2};

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
        simulate(
            &mut params,
            &mut tissues,
            1.0,
            target_depth,
            temperature,
            1.0,
            bottom_time * 60.0,
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
        println!("Reference ceiling: {}m", model.ceiling());
        let reference_ceiling = ((model.ceiling().as_meters() + 2.999) / 3.0) as u32 * 3;

        assert_eq!(first_ceiling.0, reference_ceiling);
    }

    struct TestCeiling {
        target_depth: f32,
        bottom_time: f32,
    }

    let mut test_data: Vec<TestCeiling> = vec![];
    for _i in 0..10 {
        let target_depth = rand::rng().random_range(15.0..50.0);
        let bottom_time = rand::rng().random_range(5.0..100.0);
        test_data.push(TestCeiling {
            target_depth,
            bottom_time,
        });
    }

    for test in test_data.iter() {
        compare_ceilings(test.target_depth, test.bottom_time);
    }
}

#[cfg(feature = "std")]
#[test]
fn test_ceiling_multiple_tissues_from_csv() {
    use csv::Reader;
    use std::vec::Vec;

    use crate::{run_no_deco_loop, water_vapor_pressure, FHE, FN2};
    let mut rdr = Reader::from_path("depth.csv").unwrap();
    let mut tissues = [Tissue::default(); 16];
    let mut depth: Vec<f32> = Vec::new();
    for result in rdr.records() {
        let record = result.unwrap();
        let depth_record: f32 = record[0].parse().unwrap();
        depth.push(depth_record);
    }
    let temperature = 20.0;
    let mut amb_pressure = 1.0;
    for i in 0..tissues.len() {
        tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
        tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
    }
    let mut i: u32 = 0;
    loop {
        if i == depth.len() as u32 {
            break;
        }
        amb_pressure = -depth[i as usize] / 10.0 + 1.0;
        i += 1;
        let result = run_no_deco_loop(
            &mut DiveParameters::default(),
            &mut tissues,
            amb_pressure,
            temperature,
            1.0 / 60.0,
        );
        match result {
            Ok(_) => (),
            Err(e) => {
                #[cfg(feature = "std")]
                println!("{:?}", e);
            }
        }
        let mut loop_ceiling: u32 = 0;
        for l in 0..tissues.len() {
            let result = ceiling(DiveParameters::default(), tissues[l], l);
            loop_ceiling = u32::max(loop_ceiling, result);
        }
        #[cfg(feature = "std")]
        println!("Max ceiling for tissues: {:?}", loop_ceiling);
    }
}

#[cfg(feature = "std")]
#[test]
fn test_ceiling_generalized_dive_deco_using_minutes_time_increment() {
    use dive_deco::{BuehlmannModel, DecoModel, Depth, Gas, Time};
    use rand::Rng;
    use std::vec;
    use std::vec::Vec;

    use crate::{simulate::simulate, water_vapor_pressure, FHE, FN2};

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
        simulate(
            &mut params,
            &mut tissues,
            start_amb_pressure,
            target_depth,
            temperature,
            60.0,
            bottom_time * 60.0,
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
        println!("Reference ceiling: {}m", model.ceiling());
        let reference_ceiling = ((model.ceiling().as_meters() + 2.999) / 3.0) as u32 * 3;

        assert_eq!(first_ceiling.0, reference_ceiling);
    }

    struct TestCeiling {
        target_depth: f32,
        bottom_time: f32,
    }

    let mut test_data: Vec<TestCeiling> = vec![];
    for _i in 0..10 {
        let target_depth = rand::rng().random_range(15.0..50.0);
        let bottom_time = rand::rng().random_range(5.0..100.0);
        test_data.push(TestCeiling {
            target_depth,
            bottom_time,
        });
    }

    for test in test_data.iter() {
        compare_ceilings(test.target_depth, test.bottom_time);
    }
}
