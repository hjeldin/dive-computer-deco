#[cfg(feature="std")]
use std::println;
use crate::DiveParameters;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use crate::tissue::{calculate_tissue, Tissue};

#[cfg(feature = "serde")]
use std::vec::Vec;

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
    let mut outputs = SimulationOutputs::new();
    let mut amb_pressure = starting_ambient_pressure;
    let mut depth = 0.0;
    let mut dive_time = 0.0;
    let mut descending = true;
    let mut bottom = false;

    // Define a fixed internal time step (e.g., 1 second) for consistent simulation
    let internal_step = 1.0_f32;

    // Accumulator for output recording
    let mut output_accumulator = 0.0;

    loop {
        if descending {
            // Calculate remaining time to reach target depth at descent speed
            let remaining_depth = target_depth - depth;
            let time_to_target = remaining_depth / params.descent_speed;

            // Determine current step duration (do not overshoot target depth)
            let step = internal_step.min(time_to_target);

            // Update depth and ambient pressure
            depth += params.descent_speed * step;
            amb_pressure = depth / 10.0 + 1.0;

            // Update tissues for this step (convert seconds to minutes)
            for i in 0..16 {
                tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, step / 60.0);
            }

            dive_time += step;
            output_accumulator += step;

            // Record outputs at each interval_in_seconds
            if output_accumulator >= interval_in_seconds {
                #[cfg(feature = "serde")]
                {
                    outputs.depths.push(depth);
                    outputs.pressures.push(amb_pressure);
                    outputs.tissues_per_interval.push(*tissues);
                }
                output_accumulator -= interval_in_seconds;
            }

            if depth >= target_depth {
                descending = false;
                bottom = true;
                continue;
            }
        } else if bottom {
            // Bottom phase: stay at target depth for bottom_time_seconds
            let remaining_bottom_time = bottom_time_seconds - (dive_time - (target_depth / params.descent_speed));
            if remaining_bottom_time <= 0.0 {
                break;
            }

            let step = internal_step.min(remaining_bottom_time);

            // Depth and pressure remain constant at target depth
            depth = target_depth;
            amb_pressure = depth / 10.0 + 1.0;

            // Update tissues for this step
            for i in 0..16 {
                tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, step / 60.0);
            }

            dive_time += step;
            output_accumulator += step;

            // Record outputs at each interval_in_seconds
            if output_accumulator >= interval_in_seconds {
                #[cfg(feature = "serde")]
                {
                    outputs.depths.push(depth);
                    outputs.pressures.push(amb_pressure);
                    outputs.tissues_per_interval.push(*tissues);
                }
                output_accumulator -= interval_in_seconds;
            }
        } else {
            // Ascent or other phases can be handled here if needed
            break;
        }
    }

    outputs
}