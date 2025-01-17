#[cfg(feature="std")]
use std::println;
#[cfg(feature="std")]
use std::string::ToString;
#[cfg(feature="std")]
use std::vec::Vec;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use defmt::{Format, Formatter};
use libm::powf;
use crate::{water_vapor_pressure, FHE, FN2};
use crate::zh16c::ZhL16cGf;

#[cfg(feature = "serde")]
#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Tissue {
    pub load_n2: f32,
    pub load_he: f32,
}

#[cfg(not(feature = "serde"))]
#[derive(Default, Clone, Copy, Debug)]
pub struct Tissue {
    pub load_n2: f32,
    pub load_he: f32,
}

impl Format for Tissue {
    fn format(&self, f: Formatter) {
        defmt::write!(f, "Tissue {{ load_n2: {:?}, load_he: {:?} }}", self.load_n2, self.load_he);
    }
}

// pt(t) = palv0 + R(t - 1/k) - [palv0 - pt0 - R/k] * e^(-kt)
// pt(t) -> partial pressure of the gas in the tissue at time t
// pt0 -> initial partial pressure of the gas in the tissue at t=0
// palv0 -> initial alveolar partial pressure of the gas in the mix at t=0
// k -> tissue time constant
// R -> rate of change of the partial inert gass pressure in the breathing mix in the alveoli (bar/min)
//      R = QRamb in which Q is the fraction of the inert gas and Ramb is the rate of change of the ambient pressure
// t -> time
pub fn calculate_tissue(
    mut tissue: Tissue,
    tissue_index: usize,
    amb_pressure: f32,
    temperature: f32,
    minutes_since_last_check: f32,
) -> Tissue {
    // current ambient pressure for fractions
    let ppn2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
    let pphe = (amb_pressure - water_vapor_pressure(temperature)) * FHE;

    // current tissue load for fraction
    let p0n2 = tissue.load_n2;
    let p0he = tissue.load_he;

    // half life for the tissue in minutes
    let kn2 = ZhL16cGf::N2_HALF_LIFE[tissue_index];
    let khe = ZhL16cGf::HE_HALF_LIFE[tissue_index];

    // let r = 0.8;

    // let exponent_n2 = -kn2 * minutes_since_last_check as f32;
    // let e_to_exponent_n2 = powf(2.71828, exponent_n2);

    // let r_mult_n2 = r * (minutes_since_last_check as f32 - 1.0 / kn2);
    // let inner = p0n2 - ppn2 - r / kn2;
    // let ptt = p0n2 + r_mult_n2 - inner * e_to_exponent_n2;

    let fn2 = p0n2 + (ppn2 - p0n2) * (1.0 - (1.0 / powf(2.0, minutes_since_last_check as f32 / kn2)));
    let fhe = p0he + (pphe - p0he) * (1.0 - (1.0 / powf(2.0, minutes_since_last_check as f32 / khe)));

    tissue.load_n2 = fn2;
    tissue.load_he = fhe;

    tissue
}

#[test]
fn test_calculate_tissues() {
    let mut tissues = [Tissue::default(); 16];
    let temperature = 20.0;
    let amb_pressure = 5.0;
    for i in 0..tissues.len() {
        tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
        tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
    }
    let time_since_last_check = 1.0; // minutes
    let result = calculate_tissue(
        tissues[15],
        15,
        amb_pressure,
        temperature,
        time_since_last_check,
    );
    assert_eq!(result.load_n2, 3.906155);
}

#[cfg(feature = "std")]
#[test]
fn calculate_tissues_from_csv() {
    use csv::Reader;
    use csv::Writer;

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
    let mut wtr = Writer::from_path("tissues.csv").unwrap();
    let _ = wtr.write_record(&["amb_pressure", "time", "tissue_1", "tissue_2", "tissue_3", "tissue_4", "tissue_5", "tissue_6", "tissue_7", "tissue_8", "tissue_9", "tissue_10", "tissue_11", "tissue_12", "tissue_13", "tissue_14", "tissue_15", "tissue_16"]);
    loop {
        if i == depth.len() as u32 {
            break;
        }
        amb_pressure = -depth[i as usize] / 10.0 + 1.0;
        i += 1;
        let _ = wtr.write_field(amb_pressure.to_string());
        let _ = wtr.write_field(i.to_string());
        for l in 0..tissues.len() {
            tissues[l] = calculate_tissue(tissues[l], l, amb_pressure, temperature, 1.0/60.0);
            #[cfg(feature = "std")]
            println!("{:?} - {:?} - {:?}", l, tissues[l].load_n2, amb_pressure);
            let _ = wtr.write_field(tissues[l].load_n2.to_string());
        }
        let _ = wtr.write_record(None::<&[u8]>);
    }
    let _ = wtr.flush();
}
