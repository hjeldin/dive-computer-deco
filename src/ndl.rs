#[cfg(feature="std")]
use std::println;
use crate::ceiling::ceiling;
use crate::{calculate_tissue, water_vapor_pressure, DiveParameters, Tissue, FHE, FN2};
use crate::simulate::simulate;

pub fn ndl(mut dive_parameters: &mut DiveParameters, tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) -> f32 {
    // while ceiling is 0 keep looping
    let mut bottom_time = 0.0;
    let mut max_ceiling = 0.0;
    loop {
        bottom_time += 1.0;
        for i in 0..16 {
            tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, 1.0);
            max_ceiling = f32::max(max_ceiling, ceiling(&mut dive_parameters, tissues[i], i));
        }

        if max_ceiling != 0.0 {
            return bottom_time
        }
    }
}

#[test]
fn test_ndl() {
    let mut tissues = [Tissue::default(); 16];
    let temperature = 20.0;
    let amb_pressure = 1.0;
    for i in 0..tissues.len() {
        tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
        tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
    }
    simulate(DiveParameters::default(), &mut tissues, 1.0, 40.0, 20.0, 1.0, 20.0 * 60.0);
    #[cfg(feature = "std")]
    println!("{:#?}", tissues);
    #[cfg(feature = "std")]
    println!("{}", ndl(&mut DiveParameters::default(), &mut tissues, 2.8, 20.0));
}