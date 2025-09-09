use crate::tissue::Tissue;
use crate::zh16c::ZhL16cGf;
use crate::DiveParameters;
use libm::fabsf;
#[cfg(feature = "std")]
use std::println;

#[inline(never)]
pub fn ceiling(dive_parameters: DiveParameters, tissue: Tissue, tissue_index: usize, round: bool) -> u32 {
    ceiling_with_gf(dive_parameters.gf_low, dive_parameters.gf_high, &tissue, tissue_index, 1.0, round)
}

/// Interpolates gradient factor between GF_low (at first stop) and GF_high (at surface).
fn interpolate_gf(
    gf_low: f32,
    gf_high: f32,
    ambient_pressure: f32,
    surface_pressure: f32,
    first_stop_pressure: f32,
) -> f32 {
    if (first_stop_pressure - surface_pressure).abs() < 1e-6 {
        return gf_high; // avoid div by zero
    }

    // Normalized position between surface (0.0) and first stop (1.0)
    let mut fraction = (ambient_pressure - surface_pressure) 
                     / (first_stop_pressure - surface_pressure);

    // Clamp to [0, 1] to handle rounding / overshoot
    if fraction < 0.0 {
        fraction = 0.0;
    } else if fraction > 1.0 {
        fraction = 1.0;
    }

    gf_low + (gf_high - gf_low) * fraction
}


#[inline(never)]
pub fn ceiling_with_gf(
    gf_low: f32,
    gf_high: f32,
    tissue: &Tissue,
    tissue_index: usize,
    surface_pressure: f32, // usually 1.0 bar
    round: bool,
) -> u32 {
    let first_stop_pressure = first_stop_pressure(&[tissue.clone()], surface_pressure);

    let pn2 = tissue.load_n2;
    let phe = tissue.load_he;
    let p_total = pn2 + phe;

    if p_total <= 0.0 {
        return 0;
    }

    // Bühlmann coefficients
    let an2: f32 = ZhL16cGf::N2_A[tissue_index];
    let bn2: f32 = ZhL16cGf::N2_B[tissue_index];
    let ahe: f32 = ZhL16cGf::HE_A[tissue_index];
    let bhe: f32 = ZhL16cGf::HE_B[tissue_index];

    let a = ((an2 * pn2) + (ahe * phe)) / p_total;
    let b = ((bn2 * pn2) + (bhe * phe)) / p_total;

    // Interpolate GF based on current pressure
    // Clamp denominator in case first_stop == surface
    let gf = interpolate_gf(
        gf_low,
        gf_high,
        p_total,          // current tissue tension
        surface_pressure,
        first_stop_pressure,
    );


    // Bühlmann ceiling with GF
    let denominator = (1.0 - b) * gf + b;
    if denominator.abs() < 1e-10 {
        return 0;
    }

    let result_bar = (b * p_total - gf * a * b) / denominator;

    // Convert to meters relative to surface
    let result_meters = (result_bar - surface_pressure) * 10.0;

    if result_meters < 0.0 {
        return 0;
    }

    if !round {
        return result_meters as u32;
    }

    // Round up to nearest multiple of 3 m
    ((result_meters + 2.999) / 3.0) as u32 * 3
}

/// Compute the deepest unmodified ceiling (first stop pressure) across all tissues.
/// Returns pressure in bar (absolute).
pub fn first_stop_pressure(tissues: &[Tissue], surface_pressure: f32) -> f32 {
    let mut max_ceiling_bar = surface_pressure; // at least surface

    for (i, tissue) in tissues.iter().enumerate() {
        let pn2 = tissue.load_n2;
        let phe = tissue.load_he;
        let p_total = pn2 + phe;

        if p_total <= 0.0 {
            continue;
        }

        let an2: f32 = ZhL16cGf::N2_A[i];
        let bn2: f32 = ZhL16cGf::N2_B[i];
        let ahe: f32 = ZhL16cGf::HE_A[i];
        let bhe: f32 = ZhL16cGf::HE_B[i];

        let a = ((an2 * pn2) + (ahe * phe)) / p_total;
        let b = ((bn2 * pn2) + (bhe * phe)) / p_total;

        // raw ceiling without GF
        let denom = 1.0 - b;
        if denom.abs() < 1e-10 {
            continue; // skip invalid
        }

        let ceiling_bar = (b * p_total - a * b) / denom;

        if ceiling_bar > max_ceiling_bar {
            max_ceiling_bar = ceiling_bar;
        }
    }

    max_ceiling_bar
}



#[inline(never)]
pub fn max_ceiling_with_gf(gf_low: f32, gf_high: f32, tissues: &[Tissue; 16]) -> (u32, usize) {
    let mut max_ceiling = 0;
    let mut tissue_index = 0;
    for i in 0..16 {
        let tentative_max_ceiling = ceiling_with_gf(gf_low, gf_high, &tissues[i], i, 1.0, true);
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