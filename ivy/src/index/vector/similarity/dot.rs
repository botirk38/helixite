use crate::error::Result;

pub(crate) fn compute(a: &[f32], b: &[f32]) -> Result<f32> {
    let mut dot = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
    }
    Ok(dot)
}
