use dive_computer_deco::m_value::calculate_m_values;

#[test]
fn test_calculate_m_values() {
    let amb_pressure = 5.0;
    let tissue_index = 15;
    let result = calculate_m_values(amb_pressure, tissue_index);
    assert_eq!(result, 5.4124365);
}

#[test]
fn test_calculate_m_values_for_16_tissues() {
    let amb_pressure = 1.0;
    for tissue_index in 0..16 {
        let result = calculate_m_values(amb_pressure, tissue_index);
        println!("Tissue {}: M-value = {}", tissue_index, result);
    }
    // assert_eq!(result, 5.4124365);
}