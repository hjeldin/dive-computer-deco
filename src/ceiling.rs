use crate::tissue::Tissue;
use crate::zh16c::ZhL16cGf;
use crate::DiveParameters;
use libm::fabsf;
#[cfg(feature = "std")]
use std::println;

#[inline(never)]
pub fn ceiling(dive_parameters: DiveParameters, tissue: Tissue, tissue_index: usize, round: bool) -> u32 {
    ceiling_with_gf(dive_parameters.gf_low, tissue, tissue_index, round)
}

#[inline(never)]
pub fn ceiling_with_gf(gradient_factor: f32, tissue: Tissue, tissue_index: usize, round: bool) -> u32 {
    let pn2 = tissue.load_n2;
    let phe = tissue.load_he;
    let an2: f32 = ZhL16cGf::N2_A[tissue_index];
    let bn2: f32 = ZhL16cGf::N2_B[tissue_index];

    let ahe: f32 = ZhL16cGf::HE_A[tissue_index];
    let bhe: f32 = ZhL16cGf::HE_B[tissue_index];

    let p_total = pn2 + phe;
    
    // Handle edge case where tissue has no inert gas loading
    if p_total <= 0.0 {
        return 0;
    }
    
    let a = ((an2 * pn2) + (ahe * phe)) / (p_total);
    let b = ((bn2 * pn2) + (bhe * phe)) / (p_total);

    // Calculate ceiling using the Bühlmann equation with gradient factors
    let denominator = (1.0 - b) * gradient_factor + b;
    
    // Safety check for very small denominators (shouldn't happen with valid GF values)
    if fabsf(denominator) < 1e-10 {
        #[cfg(feature = "std")]
        println!("Warning: Near-zero denominator in ceiling calculation. Using fallback calculation.");
        return 0;
    }
    
    let result_bar = (b * p_total - gradient_factor * a * b) / denominator;

    // the result is in bars, we need to convert it to meters
    let result_meters = (result_bar - 1.0) * 10.0;
    
    // Ensure we don't have negative ceilings
    if result_meters < 0.0 {
        return 0;
    }

    // round down to multiples of 3
    if !round {
        return result_meters as u32;
    }
    let ceiling = ((result_meters + 2.999) / 3.0) as u32 * 3;
    
    // #[cfg(feature = "std")]
    // println!(
    //     "Tissue: {:?} \t GF: {:.2} \t Ceil (nr): {:.5} \t Ceil: {:.5}",
    //     tissue_index + 1,
    //     gradient_factor,
    //     result_meters,
    //     ceiling
    // );
    ceiling
}

#[inline(never)]
pub fn max_ceiling_with_gf(gradient_factor: f32, tissues: &[Tissue; 16]) -> (u32, usize) {
    let mut max_ceiling = 0;
    let mut tissue_index = 0;
    for i in 0..16 {
        let tentative_max_ceiling = ceiling_with_gf(gradient_factor, tissues[i], i, true);
        if tentative_max_ceiling > max_ceiling {
            max_ceiling = tentative_max_ceiling;
            tissue_index = i;
        }
    }
    (max_ceiling, tissue_index)
}

#[inline(never)]
pub fn max_ceiling(dive_parameters: DiveParameters, tissues: &[Tissue; 16]) -> (u32, usize) {
    let mut max_ceiling = 0;
    let mut tissue_index = 0;
    for i in 0..16 {
        let tentative_max_ceiling = ceiling(dive_parameters, tissues[i], i, true);
        if tentative_max_ceiling > max_ceiling {
            max_ceiling = tentative_max_ceiling;
            tissue_index = i;
        }
    }
    (max_ceiling, tissue_index)
}

/// Binary search implementation of ceiling calculation
/// Uses binary search to find the shallowest depth where the tissue is oversaturated
#[inline(never)]
pub fn binary_ceiling(dive_parameters: DiveParameters, tissue: Tissue, tissue_index: usize, round: bool) -> u32 {
    binary_ceiling_with_gf(dive_parameters.gf_low, tissue, tissue_index, round)
}

/// Binary search implementation of ceiling calculation with custom gradient factor
#[inline(never)]
pub fn binary_ceiling_with_gf(gradient_factor: f32, tissue: Tissue, tissue_index: usize, round: bool) -> u32 {
    let pn2 = tissue.load_n2;
    let phe = tissue.load_he;
    let p_total = pn2 + phe;
    
    // Handle edge case where tissue has no inert gas loading
    if p_total <= 0.0 {
        return 0;
    }
    
    // Check if we're already oversaturated at surface (1 bar)
    if !is_oversaturated_at_depth(gradient_factor, tissue, tissue_index, 0.0) {
        return 0;
    }
    
    // Binary search parameters
    let mut low_depth = 0.0_f32;
    let mut high_depth = 100.0_f32; // Start with a reasonable ceiling estimate
    const PRECISION: f32 = 0.1; // Precision in meters
    const MAX_ITERATIONS: u32 = 50; // Safety limit
    let mut iterations = 0;
    
    // First, find an upper bound where we're not oversaturated
    while is_oversaturated_at_depth(gradient_factor, tissue, tissue_index, high_depth) && iterations < MAX_ITERATIONS {
        high_depth *= 2.0;
        iterations += 1;
        if iterations >= MAX_ITERATIONS {
            break;
        }
    }
    
    // Reset iteration counter for binary search
    iterations = 0;
    
    // Binary search for the exact ceiling
    while (high_depth - low_depth) > PRECISION && iterations < MAX_ITERATIONS {
        let mid_depth = (low_depth + high_depth) / 2.0;
        
        if is_oversaturated_at_depth(gradient_factor, tissue, tissue_index, mid_depth) {
            // Still oversaturated at mid_depth, ceiling is deeper
            low_depth = mid_depth;
        } else {
            // Not oversaturated at mid_depth, ceiling is shallower
            high_depth = mid_depth;
        }
        
        iterations += 1;
    }
    
    let result_meters = high_depth;
    
    // Ensure we don't have negative ceilings
    if result_meters < 0.0 {
        return 0;
    }

    // Round to multiples of 3 if requested
    if !round {
        return result_meters as u32;
    }
    
    ((result_meters + 2.999) / 3.0) as u32 * 3
}

/// Helper function to check if tissue is oversaturated at a given depth
fn is_oversaturated_at_depth(gradient_factor: f32, tissue: Tissue, tissue_index: usize, depth_meters: f32) -> bool {
    let pn2 = tissue.load_n2;
    let phe = tissue.load_he;
    let an2: f32 = ZhL16cGf::N2_A[tissue_index];
    let bn2: f32 = ZhL16cGf::N2_B[tissue_index];
    let ahe: f32 = ZhL16cGf::HE_A[tissue_index];
    let bhe: f32 = ZhL16cGf::HE_B[tissue_index];

    let p_total = pn2 + phe;
    
    // Handle edge case where tissue has no inert gas loading
    if p_total <= 0.0 {
        return false;
    }
    
    let a = ((an2 * pn2) + (ahe * phe)) / p_total;
    let b = ((bn2 * pn2) + (bhe * phe)) / p_total;

    // Calculate ambient pressure at the given depth
    let amb_pressure = depth_meters / 10.0 + 1.0;
    
    // Use the exact same Bühlmann equation as the analytical ceiling calculation:
    // result_bar = (b * p_total - gradient_factor * a * b) / denominator
    // where denominator = (1.0 - b) * gradient_factor + b
    // Tissue is oversaturated if amb_pressure < result_bar
    
    let denominator = (1.0 - b) * gradient_factor + b;
    
    // Safety check for very small denominators
    if fabsf(denominator) < 1e-10 {
        return false;
    }
    
    let required_pressure_bar = (b * p_total - gradient_factor * a * b) / denominator;
    
    // Tissue is oversaturated if current ambient pressure is less than required
    amb_pressure < required_pressure_bar
}