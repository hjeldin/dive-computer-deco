#[cfg(feature="std")]
use std::println;
use crate::DiveParameters;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use crate::tissue::{calculate_tissue, Tissue};

#[cfg(all(feature = "serde", feature = "std"))]
use std::vec::Vec;
#[cfg(all(feature = "serde", not(feature = "std")))]
use alloc::vec::Vec;

#[cfg(feature = "serde")]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimulationOutputs {
    pub depths: Vec<f32>,
    pub pressures: Vec<f32>,
    pub tissues_per_interval: Vec<[Tissue; 16]>,
}

#[cfg(not(feature = "serde"))]
use defmt::Format;
#[cfg(not(feature = "serde"))]
#[derive(Debug, Format, Clone, Copy)]
pub struct SimulationOutputs {}

#[cfg(feature = "serde")]
impl SimulationOutputs {
    pub fn new() -> Self {
        Self {
            depths: Vec::new(),
            pressures: Vec::new(),
            tissues_per_interval: Vec::new(),
        }
    }
}

#[cfg(not(feature = "serde"))]
impl SimulationOutputs {
    pub fn new() -> Self {
        Self {}
    }
}

#[inline(never)]
pub fn simulate(
    params: &mut DiveParameters,
    tissues: &mut [Tissue; 16],
    starting_ambient_pressure: f32,
    target_depth: f32,
    temperature: f32,
    interval_in_seconds: f32,
    bottom_time_seconds: f32,
) -> SimulationOutputs {
    simulate_with_ascent(params, tissues, starting_ambient_pressure, target_depth, temperature, interval_in_seconds, bottom_time_seconds, true)
}

#[inline(never)]
pub fn simulate_with_ascent(
    params: &mut DiveParameters,
    tissues: &mut [Tissue; 16],
    starting_ambient_pressure: f32,
    target_depth: f32,
    temperature: f32,
    interval_in_seconds: f32,
    bottom_time_seconds: f32,
    include_ascent: bool,
) -> SimulationOutputs {
    use crate::ceiling::max_ceiling_with_gf;
    
    let mut outputs = SimulationOutputs::new();
    let mut amb_pressure = starting_ambient_pressure;
    let mut depth = 0.0;
    let mut dive_time = 0.0;
    let mut descending = true;
    let mut bottom = false;
    let mut ascending = false;
    let mut at_deco_stop = false;
    let mut first_stop_depth: Option<f32> = None;
    let mut current_deco_depth = 0.0;
    let mut deco_stop_time = 0.0;
    let mut accumulated_short_stop_time = 0.0;

    // Define a fixed internal time step (e.g., 1 second) for consistent simulation
    let internal_step = 1.0_f32;

    // Accumulator for output recording
    let mut output_accumulator = 0.0;
    
    // Safety counter to prevent infinite loops
    let mut iteration_count = 0;
    const MAX_ITERATIONS: u32 = 50000000; // Increased limit for ascent phase

    #[cfg(feature = "std")]
    println!("Starting dive simulation: descent -> bottom -> ascent with decompression");

    loop {
        iteration_count += 1;
        
        // Safety check to prevent infinite loops
        if iteration_count >= MAX_ITERATIONS {
            #[cfg(feature = "std")]
            println!("⚠️  Warning: Simulation reached maximum iterations ({}). Stopping simulation.", MAX_ITERATIONS);
            break;
        }
        
        if descending {
            // DESCENT PHASE
            let remaining_depth = target_depth - depth;
            let time_to_target = remaining_depth / params.descent_speed;
            let step = internal_step.min(time_to_target);

            depth += params.descent_speed * step;
            amb_pressure = depth / 10.0 + 1.0;

            for i in 0..16 {
                tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, step / 60.0);
            }

            dive_time += step;
            output_accumulator += step;

            if output_accumulator >= interval_in_seconds {
                record_output(&mut outputs, depth, amb_pressure, tissues);
                output_accumulator -= interval_in_seconds;
            }

            if depth >= target_depth {
                #[cfg(feature = "std")]
                println!("Reached target depth: {}m after {} seconds", target_depth, dive_time);
                descending = false;
                bottom = true;
                continue;
            }
        } else if bottom {
            // BOTTOM PHASE
            let descent_time = target_depth / params.descent_speed;
            let remaining_bottom_time = bottom_time_seconds - (dive_time - descent_time);
            
            if remaining_bottom_time <= 0.0 {
                #[cfg(feature = "std")]
                println!("Bottom time completed. Starting ascent...");
                bottom = false;
                ascending = true;
                continue;
            }

            let step = internal_step.min(remaining_bottom_time);
            depth = target_depth;
            amb_pressure = depth / 10.0 + 1.0;

            for i in 0..16 {
                tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, step / 60.0);
            }

            dive_time += step;
            output_accumulator += step;

            if output_accumulator >= interval_in_seconds {
                record_output(&mut outputs, depth, amb_pressure, tissues);
                output_accumulator -= interval_in_seconds;
            }
        } else if ascending && include_ascent {
            // ASCENT PHASE WITH DECOMPRESSION STOPS
            // First, check with GF Low to determine if we need any decompression
            let (ceiling_with_gf_low, _) = max_ceiling_with_gf(params.gf_low, tissues);
            
            // Set first stop depth if not set
            if first_stop_depth.is_none() && ceiling_with_gf_low > 0 {
                first_stop_depth = Some(ceiling_with_gf_low as f32);
            }
            
            // Calculate current gradient factor based on depth
            let current_gf = if let Some(first_stop) = first_stop_depth {
                if depth <= 0.0 {
                    params.gf_high
                } else if depth >= first_stop {
                    params.gf_low
                } else {
                    // Linear interpolation: GF Low at first stop, GF High at surface
                    let depth_ratio = depth / first_stop;
                    // Correct interpolation: GF increases from GF_low to GF_high as depth decreases
                    params.gf_low + (params.gf_high - params.gf_low) * (1.0 - depth_ratio)
                }
            } else {
                // No decompression required, use GF High
                params.gf_high
            };
            
            let (current_ceiling, _controlling_tissue) = max_ceiling_with_gf(current_gf, tissues);
            
            if !at_deco_stop {
                // Check if we need a decompression stop
                if current_ceiling > 0 && depth > current_ceiling as f32 {
                    // We need to make a deco stop
                    let deco_depth = calculate_deco_stop_depth(current_ceiling);
                    current_deco_depth = deco_depth;
                    at_deco_stop = true;
                    deco_stop_time = accumulated_short_stop_time; // Start with accumulated time from skipped stops
                    accumulated_short_stop_time = 0.0; // Reset accumulator
                    
                    #[cfg(feature = "std")]
                    if deco_stop_time > 0.0 {
                        println!("Deco stop required at {}m (ceiling: {}m, controlling tissue: {}) - including {:.1}s from skipped stops", 
                                deco_depth, current_ceiling, _controlling_tissue, deco_stop_time);
                    } else {
                        println!("Deco stop required at {}m (ceiling: {}m, controlling tissue: {})", 
                                deco_depth, current_ceiling, _controlling_tissue);
                    }
                    
                    // Ascend to deco stop depth if we're deeper
                    if depth > deco_depth {
                        let depth_to_ascend = depth - deco_depth;
                        let time_to_deco_depth = depth_to_ascend / params.ascent_speed;
                        let step = internal_step.min(time_to_deco_depth);
                        
                        depth -= params.ascent_speed * step;
                        if depth <= deco_depth {
                            depth = deco_depth;
                        }
                        amb_pressure = depth / 10.0 + 1.0;
                        
                        for i in 0..16 {
                            tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, step / 60.0);
                        }
                        
                        dive_time += step;
                        output_accumulator += step;
                        
                        if output_accumulator >= interval_in_seconds {
                            record_output(&mut outputs, depth, amb_pressure, tissues);
                            output_accumulator -= interval_in_seconds;
                        }
                        continue;
                    }
                } else if depth > 0.0 {
                    // No deco stop needed, continue ascending
                    // Use a more conservative ceiling check when close to surface
                    let safety_ceiling = if depth < 6.0 {
                        // Use a more conservative GF near surface to ensure complete decompression
                        let conservative_gf = current_gf * 0.9;
                        let (conservative_ceiling, _) = max_ceiling_with_gf(conservative_gf, tissues);
                        conservative_ceiling
                    } else {
                        current_ceiling
                    };
                    
                    if safety_ceiling == 0 || depth > safety_ceiling as f32 {
                        let depth_to_surface = depth;
                        let time_to_surface = depth_to_surface / params.ascent_speed;
                        let step = internal_step.min(time_to_surface);
                        
                        depth -= params.ascent_speed * step;
                        if depth <= 0.0 {
                            depth = 0.0;
                            amb_pressure = starting_ambient_pressure;
                            
                            // Final surface phase - allow tissues to offgas to surface pressure
                            // This ensures no residual decompression obligation
                            let surface_time_needed = 60.0; // 1 minute at surface
                            let mut surface_time = 0.0;
                            
                            while surface_time < surface_time_needed {
                                for i in 0..16 {
                                    tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, internal_step / 60.0);
                                }
                                
                                dive_time += internal_step;
                                surface_time += internal_step;
                                output_accumulator += internal_step;
                                
                                if output_accumulator >= interval_in_seconds {
                                    record_output(&mut outputs, depth, amb_pressure, tissues);
                                    output_accumulator -= interval_in_seconds;
                                }
                            }
                            
                            #[cfg(feature = "std")]
                            println!("Reached surface! Total dive time: {:.1} minutes", dive_time / 60.0);
                            break;
                        }
                        amb_pressure = depth / 10.0 + 1.0;
                        
                        for i in 0..16 {
                            tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, step / 60.0);
                        }
                        
                        dive_time += step;
                        output_accumulator += step;
                        
                        if output_accumulator >= interval_in_seconds {
                            record_output(&mut outputs, depth, amb_pressure, tissues);
                            output_accumulator -= interval_in_seconds;
                        }
                    } else {
                        // Need to wait at current depth - ceiling still constrains us
                        amb_pressure = depth / 10.0 + 1.0;
                        
                        for i in 0..16 {
                            tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, internal_step / 60.0);
                        }
                        
                        dive_time += internal_step;
                        output_accumulator += internal_step;
                        
                        if output_accumulator >= interval_in_seconds {
                            record_output(&mut outputs, depth, amb_pressure, tissues);
                            output_accumulator -= interval_in_seconds;
                        }
                    }
                } else {
                    // We're at the surface
                    break;
                }
            } else {
                // AT DECOMPRESSION STOP
                depth = current_deco_depth;
                amb_pressure = depth / 10.0 + 1.0;
                
                // Update tissues while at deco stop
                for i in 0..16 {
                    tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, internal_step / 60.0);
                }
                
                dive_time += internal_step;
                deco_stop_time += internal_step;
                output_accumulator += internal_step;
                
                if output_accumulator >= interval_in_seconds {
                    record_output(&mut outputs, depth, amb_pressure, tissues);
                    output_accumulator -= interval_in_seconds;
                }
                
                // Check if we can leave the deco stop (ceiling has cleared)
                let (new_ceiling, _) = max_ceiling_with_gf(current_gf, tissues);
                
                // Check if we can leave this deco stop
                if deco_stop_time >= 60.0 && (new_ceiling == 0 || new_ceiling as f32 + 0.5 < current_deco_depth) {
                    #[cfg(feature = "std")]
                    println!("Completed deco stop at {}m after {:.1} minutes", current_deco_depth, deco_stop_time / 60.0);
                    at_deco_stop = false;
                    deco_stop_time = 0.0;
                } else if deco_stop_time < 60.0 && (new_ceiling == 0 || new_ceiling as f32 + 0.5 < current_deco_depth) {
                    // This stop would be less than 1 minute - accumulate the time and move to next stop
                    accumulated_short_stop_time += deco_stop_time;
                    #[cfg(feature = "std")]
                    println!("Skipping short deco stop at {}m ({:.1}s) - adding to next stop", current_deco_depth, deco_stop_time);
                    at_deco_stop = false;
                    deco_stop_time = 0.0;
                }
                
                // Safety check - max deco stop time of 20 minutes
                if deco_stop_time >= 20.0 * 60.0 {
                    #[cfg(feature = "std")]
                    println!("⚠️  Maximum deco stop time reached at {}m. Continuing ascent.", current_deco_depth);
                    at_deco_stop = false;
                    deco_stop_time = 0.0;
                }
            }
        } else {
            // End of simulation (no ascent requested)
            break;
        }
    }

    #[cfg(feature = "std")]
    println!("Simulation completed after {} iterations", iteration_count);

    outputs
}

fn calculate_deco_stop_depth(ceiling: u32) -> f32 {
    // Round up to the next 3m increment, with a minimum depth of 3m
    let deco_depth = ((ceiling as f32 + 2.999) / 3.0) as u32 as f32 * 3.0;
    deco_depth.max(3.0)
}

#[cfg(feature = "serde")]
fn record_output(outputs: &mut SimulationOutputs, depth: f32, pressure: f32, tissues: &[Tissue; 16]) {
    outputs.depths.push(depth);
    outputs.pressures.push(pressure);
    outputs.tissues_per_interval.push(*tissues);
}

#[cfg(not(feature = "serde"))]
fn record_output(_outputs: &mut SimulationOutputs, _depth: f32, _pressure: f32, _tissues: &[Tissue; 16]) {
    // No-op for non-serde builds
}