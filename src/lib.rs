#![no_std]

#[cfg(feature="std")]
use std::println;

use defmt::Format;

#[cfg(feature = "std")]
extern crate std;

pub mod ceiling;
pub mod ndl;
pub mod simulate;
pub mod m_value;
pub mod tissue;
pub mod zh16c;

pub struct Gas {
    pub n2: f32,
    pub he: f32,
}

#[derive(Debug, Format, Copy, Clone)]
pub struct DiveParameters {
    pub descent_speed: f32,                 // m/s
    pub ascent_speed: f32,                  // m/s
    pub safety_stop_ascent_speed: f32,      // m/s
    pub safety_stop_duration: f32,          // s
    pub safety_stop_depth: f32,             // m
    pub gf_low: f32,                        // 0 < x <= 1
    pub gf_high: f32,                       // 0 < x <= 1
    pub sac_rate: f32,                      // litres per minute
}

impl DiveParameters {
    pub fn new(gf_high: f32, gf_low: f32) -> Self {
        DiveParameters {
            descent_speed: 0.333333333,
            ascent_speed: 0.1666667,
            safety_stop_ascent_speed: 0.083,
            safety_stop_duration: 3.0,
            safety_stop_depth: 5.0,
            gf_low,
            gf_high,
            sac_rate: 20.0
        }
    }
}

impl Default for DiveParameters {
    fn default() -> Self {
        DiveParameters {
            descent_speed: 0.33,
            ascent_speed: 0.17,
            safety_stop_ascent_speed: 0.083,
            safety_stop_duration: 3.0,
            safety_stop_depth: 5.0,
            gf_low: 1.0,
            gf_high: 1.0,
            sac_rate: 20.0
        }
    }
}

pub const FN2: f32 = 0.79;
pub const FHE: f32 = 0.0;

// pub enum Gas {
//     N2,
//     He,
// }

use crate::ceiling::max_ceiling;
use crate::m_value::calculate_m_values;
use crate::tissue::{calculate_tissue, Tissue};



pub fn default_tissue_load(temperature: f32) -> f32 {
    0.79 * (1.0 - water_vapor_pressure(temperature))
}

// at 37 deg celsius should return 0.0627 bar (47 mmHg)
pub fn water_vapor_pressure(_temperature: f32) -> f32 {
    // Water vapor pressure in bar
    // Antoine equation
    // P = 10^(A - B / (C + T))
    // P -> water vapor pressure in bar
    // T -> temperature in Celsius
    // A, B, C -> Antoine equation constants
    // A = 8.07131, B = 1730.63, C = 233.426
    // 0.0555
    0.0627
}

#[test]
fn test_water_vapor_pressure() {
    assert_eq!(water_vapor_pressure(37.0), 0.0555);
}


#[derive(Debug, Format)]
pub enum DecoError {
    Oversaturation,
    BurstCeiling,
    InvalidSolution,
}


pub fn run_no_deco_loop(_dive_parameters: &mut DiveParameters, tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32, delta_t: f32) -> Result<(), DecoError> {
    for i in 0..16 {
        tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, delta_t);
        defmt::info!("{:?} - {:?}", i, tissues[i].load_n2);

        if calculate_m_values(amb_pressure, i) < tissues[i].load_n2 {
            return Err(DecoError::Oversaturation);
        }
    }

    Ok(())
}

pub fn calculate_deco_stops(dive_parameters: DiveParameters, tissues: &mut [Tissue; 16], _amb_pressure: f32, temperature: f32) -> Result<(), DecoError> {
    let first_stop = max_ceiling(dive_parameters, tissues);
    let last_stop: i32 = 3;
    #[cfg(feature = "std")]
    println!("Deco starting tissues {:?}", tissues);
    let mut current_stop_depth = first_stop.0;
    let mut base_tissues_clone = tissues.clone();
    loop {
        if current_stop_depth >= last_stop as u32 {
            let mut stop_length = 0;
            // clone current stop depth tissues. first iteration will be the same as tissues, then it will be the previous stop tissues
            let mut tissues_clone = base_tissues_clone.clone();
            loop {
                // assume that the ambient pressure keeps constant for the stop duration
                stop_length += 1;
                // 166 minutes of air is beyond what I can currently carry in my tanks
                if stop_length > 10000 {
                    return Err(DecoError::InvalidSolution);
                }
                let current_ceiling = max_ceiling(dive_parameters, &tissues_clone);
                // if the currently calculated ceiling is less than the current stop depth, we may proceed to the next stop
                // clone the tissues associated with this stop
                if current_ceiling.0 < current_stop_depth as u32 {
                    #[cfg(feature = "std")]
                    println!("Deco stop at {:?} for {:?} seconds", current_stop_depth, stop_length);
                    base_tissues_clone = tissues_clone.clone();
                    break;
                }

                // otherwise, clone the tissues and let them desaturate for another second
                for i in 0..16 {
                    tissues_clone[i] = calculate_tissue(tissues_clone[i], i, current_stop_depth as f32 / 10.0 + 1.0, temperature, 1.0/60.0);
                }
            }
            // deco stop complete, proceed to next stop
            current_stop_depth -= 3;
        } else {
            break;
        }
    }

    Ok(())
}

#[cfg(feature = "std")]
#[test]
fn test_deco_stops() {
    use crate::simulate::simulate;
    let mut tissues = [Tissue::default(); 16];
    let temperature = 20.0;
    let amb_pressure = 1.0;
    for i in 0..tissues.len() {
        tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
        tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
    }

    let simulation = simulate(&mut DiveParameters::default(), &mut tissues, 1.0, 50.0, temperature, 1.0, 20.0 * 60.0);
    println!("{:?}", simulation);

    let _result = calculate_deco_stops(DiveParameters::default(), &mut tissues, amb_pressure, temperature);
}

#[cfg(feature = "std")]
#[test]
fn test_deco_loop() {
    use csv::Reader;
    use std::vec::Vec;
    let mut rdr = Reader::from_path("depth.csv").unwrap();
    let mut tissues = [Tissue::default(); 16];
    let mut depth: Vec<f32> = Vec::new();
    for result in rdr.records(){
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
        let result = run_no_deco_loop(&mut DiveParameters::default(), &mut tissues, amb_pressure, temperature, 1.0/60.0);
        match result {
            Ok(_) => (),
            Err(e) => {
                println!("DECO NOW MANDATORY: {:?}", e);
                panic!("wtf");
            }
        }
    }
}