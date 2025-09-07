use dive_computer_deco::{tissue::{calculate_tissue, Tissue}, water_vapor_pressure, FHE, FN2};

#[test]
fn test_calculate_tissue_no_change() {
    let amb_pressure = 3.0;
    let temperature = 20.0;
    let time_since_last_check = 0.0; // no time has passed

    let tissue = Tissue {
        load_n2: amb_pressure * FN2,
        load_he: amb_pressure * FHE,
    };
    let result = calculate_tissue(tissue, 0, amb_pressure, temperature, time_since_last_check);

    assert_eq!(result.load_n2, tissue.load_n2);
    assert_eq!(result.load_he, tissue.load_he);
}

#[test]
fn test_calculate_tissue_with_time() {
    let tissue = Tissue {
        load_n2: 2.0,
        load_he: 1.0,
    };
    let amb_pressure = 4.0;
    let temperature = 20.0;
    let time_since_last_check = 1.0; // 1 minute has passed

    let result = calculate_tissue(tissue, 0, amb_pressure, temperature, time_since_last_check);

    assert!(result.load_n2 > tissue.load_n2);
    // assert!(result.load_he > tissue.load_he);
}

#[test]
fn test_calculate_tissue_with_zero_ambient_pressure() {
    let tissue = Tissue {
        load_n2: 2.0,
        load_he: 1.0,
    };
    let amb_pressure = 0.1; // unrealistic, but for edge case testing
    let temperature = 20.0;
    let time_since_last_check = 1.0;

    let result = calculate_tissue(tissue, 0, amb_pressure, temperature, time_since_last_check);

    assert!(result.load_n2 < tissue.load_n2);
    // assert!(result.load_he < tissue.load_he);
}

#[test]
fn test_calculate_tissue_with_high_ambient_pressure() {
    let tissue = Tissue {
        load_n2: 2.0,
        load_he: 1.0,
    };
    let amb_pressure = 10.0; // high ambient pressure
    let temperature = 20.0;
    let time_since_last_check = 1.0;

    let result = calculate_tissue(tissue, 0, amb_pressure, temperature, time_since_last_check);

    assert!(result.load_n2 > tissue.load_n2);
    // assert!(result.load_he > tissue.load_he);
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
    assert_eq!(result.load_n2, 3.9004672);
}

#[cfg(feature = "std")]
#[test]
fn test_calculate_multi_tissues() {
    use std::println;

    let mut tissues = [Tissue::default(); 16];
    let depth = 40.0;
    let amb_pressure = depth / 10.0 + 1.0;
    let temperature = 20.0;
    let bottom_time = 2; // minutes

    for i in 0..tissues.len() {
        tissues[i].load_n2 = 1.0 * FN2;
        tissues[i].load_he = 1.0 * FHE;
    }
    let time_since_last_check = 1.0; // minutes
    for minutes in 1..=bottom_time {
        println!("=========== minute {} ===========", minutes);
        for i in 0..tissues.len() {
            let result = calculate_tissue(
                tissues[i],
                i,
                amb_pressure,
                temperature,
                time_since_last_check,
            );

            tissues[i] = result;
            println!("{:?}", result);
        }
    }
}

#[cfg(feature = "std")]
#[test]
fn calculate_tissues_from_csv() {
    use csv::Reader;
    use csv::Writer;
    use std::println;
    use std::string::ToString;
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
