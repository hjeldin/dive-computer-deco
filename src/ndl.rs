use crate::ceiling::{binary_ceiling_with_gf, ceiling};
use crate::tissue::{calculate_tissue, Tissue};
use crate::DiveParameters;
use core::panic;

pub fn ndl(
    dive_parameters: DiveParameters,
    tissues: &mut [Tissue; 16],
    amb_pressure: f32,
    temperature: f32,
) -> f32 {
    // while ceiling is 0 keep looping
    let mut bottom_time = 0.0;
    let mut max_ceiling: u32 = 0;
    const MAX_ITERATIONS: u32 = 10000; // Prevent infinite loops with extreme GF values
    let mut iterations = 0;

    loop {
        max_ceiling = 0; // Reset max_ceiling at the start of each iteration
        for i in 0..16 {
            tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, 1.0);
            max_ceiling = u32::max(max_ceiling, ceiling(dive_parameters, tissues[i], i, true));
        }

        if max_ceiling != 0 {
            return bottom_time;
        }

        bottom_time += 1.0;
        iterations += 1;

        // Safety check to prevent infinite loops with extreme gradient factors
        if iterations >= MAX_ITERATIONS {
            panic!("NDL calculation exceeded maximum iterations");
        }
    }
}

/// Binary search implementation of NDL (No Decompression Limit) calculation
/// Uses binary search to find the maximum bottom time where ceiling remains 0
pub fn binary_ndl(
    dive_parameters: DiveParameters,
    tissues: &mut [Tissue; 16],
    amb_pressure: f32,
    temperature: f32,
) -> f32 {
    let mut bottom_time = 0.0;
    let mut max_ceiling: u32 = 0;
    const MAX_ITERATIONS: u32 = 10000; // Prevent infinite loops with extreme GF values
    let mut iterations = 0;
    loop {
        max_ceiling = 0; // Reset max_ceiling at the start of each iteration
        for i in 0..16 {
            tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, 1.0);
            max_ceiling = u32::max(max_ceiling, binary_ceiling_with_gf(dive_parameters.gf_low, tissues[i], i, true));
        }

        if max_ceiling != 0 {
            return bottom_time;
        }

        bottom_time += 1.0;
        iterations += 1;

        // Safety check to prevent infinite loops with extreme gradient factors
        if iterations >= MAX_ITERATIONS {
            panic!("NDL calculation exceeded maximum iterations");
        }
    }
}