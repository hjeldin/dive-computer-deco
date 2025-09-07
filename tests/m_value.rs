use dive_computer_deco::m_value::calculate_m_values;

#[test]
fn test_calculate_m_values() {
    let amb_pressure = 5.0;
    let tissue_index = 15;
    let result = calculate_m_values(amb_pressure, tissue_index);
    assert_eq!(result, 5.4124365);
}

#[cfg(feature = "std")]
#[test]
fn plot_m_values() {
    use csv::Writer;
    use std::string::ToString;
    let mut wtr = Writer::from_path("test_output/m_values.csv").unwrap();
    let _ = wtr.write_record(&[
        "amb_pressure",
        "m_value_1",
        "m_value_2",
        "m_value_3",
        "m_value_4",
        "m_value_5",
        "m_value_6",
        "m_value_7",
        "m_value_8",
        "m_value_9",
        "m_value_10",
        "m_value_11",
        "m_value_12",
        "m_value_13",
        "m_value_14",
        "m_value_15",
        "m_value_16",
    ]);

    // iterate for pressures in step of 0.5 bar
    for press in 2..9 {
        let _ = wtr.write_field((press as f32 / 2.0 as f32).to_string());
        for i in 0..16 {
            let _ = wtr.write_field(calculate_m_values(press as f32 / 2.0 as f32, i).to_string());
        }
        let _ = wtr.write_record(None::<&[u8]>);
    }
    let _ = wtr.flush();
}
