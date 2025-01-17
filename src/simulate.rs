#[cfg(feature="std")]
use std::println;
use defmt::Format;
use crate::{DiveParameters};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use crate::tissue::{calculate_tissue, Tissue};

#[cfg(feature = "serde")]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SimulationOutputs {
    pub depths: Vec<f32>,
    pub pressures: Vec<f32>,
    pub tissues_per_interval: Vec<[Tissue; 16]>,
}

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

pub fn simulate(params: &mut DiveParameters, tissues: &mut [Tissue; 16], starting_ambient_pressure: f32, target_depth: f32, temperature: f32, interval_in_seconds: f32, bottom_time_seconds: f32) -> SimulationOutputs {
    let outputs = SimulationOutputs::new();
    let mut amb_pressure = starting_ambient_pressure;
    let mut depth = 0.0;
    let mut dive_time = 0.0;
    let mut descending = true;
    let mut bottom = false;
    let mut descent_time = 0.0;
    loop {
        if descending {
            if depth >= target_depth {
                #[cfg(feature = "std")]
                println!("Reached target depth after {}s", dive_time);
                descending = false;
                bottom = true;
            }
            depth += params.descent_speed * interval_in_seconds;
            amb_pressure = depth / 10.0 + 1.0;
            dive_time += interval_in_seconds;
            descent_time += interval_in_seconds;
            #[cfg(feature = "serde")]
            if(dive_time % 60.0 == 0.0) {
                outputs.depths.push(depth);
                outputs.pressures.push(amb_pressure);
            }
            #[cfg(feature = "serde")]
            let mut instantTissues = [Tissue::default(); 16];
            for i in 0..16 {
                tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, interval_in_seconds/60.0);
                #[cfg(feature = "serde")]
                if(dive_time % 60.0 == 0.0) {
                    instantTissues[i] = tissues[i];
                }
            }
            #[cfg(feature = "serde")]
            if(dive_time % 60.0 == 0.0) {
                outputs.tissues_per_interval.push(instantTissues);
            }
        }
        if bottom {
            dive_time += interval_in_seconds;
            if dive_time >= bottom_time_seconds {
                #[cfg(feature = "std")]
                println!("Reached ascent phase after {}s", dive_time);
                break;
            }
            #[cfg(feature = "serde")]
            if(dive_time % 60.0 == 0.0) {
                outputs.depths.push(depth);
                outputs.pressures.push(amb_pressure);
            }
            #[cfg(feature = "serde")]
            let mut instantTissues = [Tissue::default(); 16];
            for i in 0..16 {
                tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, 1.0/60.0);
                #[cfg(feature = "serde")]
                if(dive_time % 60.0 == 0.0) {
                    instantTissues[i] = tissues[i];
                }
            }
            #[cfg(feature = "serde")]
            if(dive_time % 60.0 == 0.0) {
                outputs.tissues_per_interval.push(instantTissues);
            }
        }
    }

    outputs
}