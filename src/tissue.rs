#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use defmt::{Format, Formatter};
use libm::{logf, powf};
use crate::{water_vapor_pressure, FHE, FN2};
use crate::zh16c::ZhL16cGf;

#[cfg(feature = "serde")]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Tissue {
    pub load_n2: f32,
    pub load_he: f32,
}

#[cfg(not(feature = "serde"))]
#[derive(Clone, Copy, Debug)]
pub struct Tissue {
    pub load_n2: f32,
    pub load_he: f32,
}

impl Default for Tissue {
    fn default() -> Self {
        Tissue {
            load_n2: 1.0,
            load_he: 1.0,
        }
    }
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

    assert!(minutes_since_last_check >= 0.0, "minutes_since_last_check must be >= 0.0");
    // current ambient pressure for fractions
    let ppn2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
    let pphe = (amb_pressure - water_vapor_pressure(temperature)) * FHE;

    // current tissue load for fraction
    let p0n2 = tissue.load_n2;
    let p0he = tissue.load_he;

    // half life for the tissue in minutes
    let kn2 = ZhL16cGf::N2_HALF_LIFE[tissue_index];
    let khe = ZhL16cGf::HE_HALF_LIFE[tissue_index];

    let k_n2 = logf(2.0) / kn2;
    let k_he = logf(2.0) / khe;

    let e_to_exponent_n2 = powf(core::f32::consts::E, -k_n2 * minutes_since_last_check);
    let e_to_exponent_he = powf(core::f32::consts::E, -k_he * minutes_since_last_check);

    let fn2 = ppn2 + (p0n2 - ppn2) * e_to_exponent_n2;
    let fhe = p0he + (pphe - p0he) * e_to_exponent_he;

    tissue.load_n2 = fn2;
    tissue.load_he = fhe;

    tissue
}