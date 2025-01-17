#[cfg(feature="std")]
use std::println;
#[cfg(feature="std")]
use std::vec::Vec;
use crate::ceiling::ceiling;
use crate::{water_vapor_pressure, DiveParameters, FHE, FN2};
use crate::simulate::simulate;
use crate::tissue::{calculate_tissue, Tissue};

pub fn ndl(dive_parameters: DiveParameters, tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) -> f32 {
    // while ceiling is 0 keep looping
    let mut bottom_time = 0.0;
    let mut max_ceiling: u32 = 0;
    loop {
        for i in 0..16 {
            tissues[i] = calculate_tissue(tissues[i], i, amb_pressure, temperature, 1.0);
            max_ceiling = u32::max(max_ceiling, ceiling(dive_parameters, tissues[i], i));
        }

        if max_ceiling != 0 {
            return bottom_time
        }
        bottom_time += 1.0;
    }
}

#[cfg(feature = "std")]
#[test]
fn test_ndl() {

    fn reset_tissues(tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) {
        for i in 0..tissues.len() {
            tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
            tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
        }
    }

    let mut tissues = [Tissue::default(); 16];
    let temperature = 20.0;
    let start_amb_pressure = 1.0;

    struct TableNDL {
        depth: f32,
        ndl: f32,
        deco: f32,
    }

    let table_ndl = [
        TableNDL { depth: 60.0, ndl: 0.0, deco: 5.0 },
        TableNDL { depth: 57.0, ndl: 5.0, deco: 10.0 },
        TableNDL { depth: 54.0, ndl: 5.0, deco: 10.0 },
        TableNDL { depth: 51.0, ndl: 5.0, deco: 10.0 },
        TableNDL { depth: 48.0, ndl: 5.0, deco: 10.0 },
        TableNDL { depth: 45.0, ndl: 5.0, deco: 10.0 },
        TableNDL { depth: 42.0, ndl: 10.0, deco: 15.0 },
        TableNDL { depth: 39.0, ndl: 10.0, deco: 15.0 },
        TableNDL { depth: 36.0, ndl: 15.0, deco: 20.0 },
        TableNDL { depth: 33.0, ndl: 20.0, deco: 25.0 },
        TableNDL { depth: 30.0, ndl: 25.0, deco: 30.0 },
        TableNDL { depth: 27.0, ndl: 30.0, deco: 40.0 },
        TableNDL { depth: 24.0, ndl: 40.0, deco: 50.0 },
        TableNDL { depth: 21.0, ndl: 50.0, deco: 60.0 },
        TableNDL { depth: 18.0, ndl: 60.0, deco: 70.0 },
        TableNDL { depth: 15.0, ndl: 100.0, deco: 110.0 },
        // TableNDL { depth: 12.0, ndl: 200.0, deco: 220.0 },
    ];

    let mut results: Vec<(bool, f32)> = Vec::new();

    let mut params = DiveParameters::new(0.7, 1.0);

    for i in 0..table_ndl.len() {
        reset_tissues(&mut tissues, start_amb_pressure, temperature);
        simulate(&mut params, &mut tissues, 1.0, table_ndl[i].depth, 20.0, 1.0, 0.0 * 60.0);
        let amb_pressure = table_ndl[i].depth / 10.0 + 1.0;
        let result = ndl(params, &mut tissues, amb_pressure, 20.0);
        let result_between = result >= table_ndl[i].ndl && result <= table_ndl[i].deco;
        #[cfg(feature = "std")]
        println!("{} - {}", result, result_between);
        results.push((result_between, result));
    }

    for i in 0..results.len() {
        #[cfg(feature = "std")]
        println!("Depth: {}m - Match? {} - Table NDL: {}min - NDL: {}min - Table Deco: {}min", table_ndl[i].depth, results[i].0, table_ndl[i].ndl, results[i].1, table_ndl[i].deco);
    }

}


// #[test]
// fn test_ndl2() {
//
//     fn reset_tissues(tissues: &mut [Tissue; 16], amb_pressure: f32, temperature: f32) {
//         for i in 0..tissues.len() {
//             tissues[i].load_n2 = (amb_pressure - water_vapor_pressure(temperature)) * FN2;
//             tissues[i].load_he = (amb_pressure - water_vapor_pressure(temperature)) * FHE;
//         }
//     }
//
//     let mut tissues = [Tissue::default(); 16];
//     let temperature = 20.0;
//     let start_amb_pressure = 1.0;
//
//     struct TableNDL {
//         depth: f32,
//         ndl: f32,
//         deco: f32,
//     }
//
//     let table_ndl = [
//         TableNDL { depth: 57.0, ndl: 5.0, deco: 10.0 },
//     ];
//
//     let mut params = DiveParameters::new(1.0, 1.0);
//
//     for i in 0..table_ndl.len() {
//         reset_tissues(&mut tissues, start_amb_pressure, temperature);
//         simulate(&mut params, &mut tissues, 1.0, table_ndl[i].depth, 20.0, 1.0, 0.0 * 60.0);
//         let amb_pressure = table_ndl[i].depth / 10.0 + 1.0;
//         let result = ndl(params, &mut tissues, amb_pressure, 20.0);
//         let result_between = result >= table_ndl[i].ndl && result <= table_ndl[i].deco;
//         #[cfg(feature = "std")]
//         println!("{} - {}", result, result_between);
//         assert!(result_between);
//     }
//
// }
