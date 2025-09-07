use crate::zh16c::ZhL16cGf;

pub fn calculate_m_values(amb_pressure: f32, tissue_index: usize) -> f32 {
    amb_pressure / ZhL16cGf::N2_B[tissue_index] + ZhL16cGf::N2_A[tissue_index]
}