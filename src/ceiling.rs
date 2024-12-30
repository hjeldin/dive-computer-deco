#[cfg(feature="std")]
use std::println;
#[cfg(feature="std")]
use std::vec::Vec;
use libm::round;
use crate::{run_deco_loop, water_vapor_pressure, DiveParameters, Tissue, FHE, FN2, ZH_L16C_GF};

pub fn ceiling(dive_parameters: &mut DiveParameters, tissue: Tissue, tissue_index: usize) -> f32 {
    let pn2 = tissue.load_n2;
    let phe = tissue.load_he;
    let an2: f32 = ZH_L16C_GF::N2_A[tissue_index];
    let bn2: f32 = ZH_L16C_GF::N2_B[tissue_index];

    let ahe: f32 = ZH_L16C_GF::HE_A[tissue_index];
    let bhe: f32 = ZH_L16C_GF::HE_B[tissue_index];

    let a = (an2 * pn2 + ahe * phe) / (pn2 + phe);
    let b = (bn2 * pn2 + bhe * phe) / (pn2 + phe);

    let r = ((pn2 + phe) - a * dive_parameters.gf_high) * (b / (dive_parameters.gf_high - (dive_parameters.gf_high * b) + b));

    #[cfg(feature = "std")]
    println!("{} - r: {}", tissue_index, r);
    // round down to multiples of 3
    let ceiling = round((r / 3.0) as f64);
    (ceiling * 3.0) as f32
}

#[test]
fn test_ceiling() {
    let tissue = Tissue {
        load_n2: 17.65,
        load_he: 0.0,
    };

    let tissue_index = 1;
    let result = ceiling(&mut DiveParameters::default(), tissue, tissue_index);
    assert_eq!(result, 12.0);
}

#[test]
fn test_ceiling_gf() {
    let tissue = Tissue {
        load_n2: 17.65,
        load_he: 0.0,
    };

    let params = &mut DiveParameters::default();
    params.gf_high = 0.3;
    params.gf_low = 0.3;

    let tissue_index = 1;
    let result = ceiling(params, tissue, tissue_index);
    assert_eq!(result, 15.0);
}

#[cfg(feature = "std")]
#[test]
fn test_ceiling_multiple_tissues_from_csv() {
    use csv::Reader;
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
        run_deco_loop(&mut tissues, amb_pressure, temperature,  1.0/60.0);
        let mut loop_ceiling = 0.0;
        for l in 0..tissues.len() {
            let result = ceiling(&mut DiveParameters::default(), tissues[l], l);
            loop_ceiling = f32::max(loop_ceiling, result);
        }
        #[cfg(feature = "std")]
        println!("Max ceiling for tissues: {:?}", loop_ceiling);
    }
}
