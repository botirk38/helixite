use crate::error::Result;

pub(crate) fn compute(a: &[f32], b: &[f32]) -> Result<f32> {
    let mut sum = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        let diff = x - y;
        sum += diff * diff;
    }
    Ok(sum.sqrt())
}
