#[cfg(feature="std")]
use std::println;
#[cfg(feature="std")]
use std::vec::Vec;
use libm::round;
use crate::{run_no_deco_loop, water_vapor_pressure, DiveParameters, FHE, FN2};
use crate::tissue::Tissue;
use crate::zh16c::ZhL16cGf;

pub fn ceiling(dive_parameters: DiveParameters, tissue: Tissue, tissue_index: usize) -> u32 {
    let pn2 = tissue.load_n2;
    let phe = tissue.load_he;
    let an2: f32 = ZhL16cGf::N2_A[tissue_index];
    let bn2: f32 = ZhL16cGf::N2_B[tissue_index];

    let ahe: f32 = ZhL16cGf::HE_A[tissue_index];
    let bhe: f32 = ZhL16cGf::HE_B[tissue_index];

    let p_total = pn2 + phe;
    let a = ((an2 * pn2) + (ahe * phe)) / (p_total);
    let b = ((bn2 * pn2) + (bhe * phe)) / (p_total);

    // let r = ((p_total) - a * dive_parameters.gf_high) * (b / (dive_parameters.gf_high - (dive_parameters.gf_high * b) + b));
    let mut result_bar = (p_total) - a * dive_parameters.gf_high;
    result_bar /= (dive_parameters.gf_high / b) + 1.0 - dive_parameters.gf_high;

    // the result is in bars, we need to convert it to meters
    let result_meters = (result_bar - 1.0) * 10.0;

    // round down to multiples of 3
    let ceiling = result_meters / 3.0;
    let rounded_ceiling = (ceiling * 3.0) as f32;
    #[cfg(feature = "std")]
    println!("{};{};{};{}", tissue_index, pn2, result_meters, rounded_ceiling);
    (ceiling * 3.0) as u32
}

pub fn max_ceiling(dive_parameters: DiveParameters, tissues: &[Tissue; 16]) -> (u32, usize) {
    let mut max_ceiling = 0;
    let mut tissue_index = 0;
    for i in 0..16 {
        if ceiling(dive_parameters, tissues[i], i) > max_ceiling {
            max_ceiling = ceiling(dive_parameters, tissues[i], i);
            tissue_index = i;
        }
    }
    (max_ceiling, tissue_index)
}

#[test]
fn test_ceiling() {
    let tissue = Tissue {
        load_n2: 17.65,
        load_he: 0.0,
    };

    let tissue_index = 1;
    let result = ceiling(DiveParameters::default(), tissue, tissue_index);
    assert_eq!(result, 12);
}

#[test]
fn test_ceiling_gf() {
    let tissue = Tissue {
        load_n2: 17.65,
        load_he: 0.0,
    };

    let params = DiveParameters::new(0.3, 0.3);

    let tissue_index = 1;
    let result = ceiling(params, tissue, tissue_index);
    assert_eq!(result, 15);
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
        let result = run_no_deco_loop(&mut DiveParameters::default(), &mut tissues, amb_pressure, temperature, 1.0/60.0);
        match result {
            Ok(_) => (),
            Err(e) => {
                #[cfg(feature = "std")]
                println!("{:?}", e);
            }
        }
        let mut loop_ceiling: u32 = 0;
        for l in 0..tissues.len() {
            let result = ceiling(DiveParameters::default(), tissues[l], l);
            loop_ceiling = u32::max(loop_ceiling, result);
        }
        #[cfg(feature = "std")]
        println!("Max ceiling for tissues: {:?}", loop_ceiling);
    }
}
