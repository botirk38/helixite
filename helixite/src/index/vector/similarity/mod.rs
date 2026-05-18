mod cosine;
mod dot;
mod euclidean;

use crate::error::{HelixiteError, Result};

pub type SimilarityFn = fn(&[f32], &[f32]) -> Result<f32>;

#[derive(Debug, Clone, Copy)]
pub enum SimilarityKind {
    Cosine,
    DotProduct,
    Euclidean,
    Custom(SimilarityFn),
}

impl SimilarityKind {
    pub(crate) fn compute(&self, a: &[f32], b: &[f32]) -> Result<f32> {
        if a.len() != b.len() {
            return Err(HelixiteError::InvalidVectorDim {
                expected: a.len(),
                actual: b.len(),
            });
        }
        match self {
            SimilarityKind::Cosine => cosine::compute(a, b),
            SimilarityKind::DotProduct => dot::compute(a, b),
            SimilarityKind::Euclidean => euclidean::compute(a, b),
            SimilarityKind::Custom(f) => f(a, b),
        }
    }

    pub(crate) fn is_higher_better(&self) -> bool {
        match self {
            SimilarityKind::Cosine => true,
            SimilarityKind::DotProduct => true,
            SimilarityKind::Euclidean => false,
            SimilarityKind::Custom(_) => true,
        }
    }

    pub(crate) fn to_byte(self) -> u8 {
        match self {
            SimilarityKind::Cosine => 0,
            SimilarityKind::DotProduct => 1,
            SimilarityKind::Euclidean => 2,
            SimilarityKind::Custom(_) => 3,
        }
    }

    pub(crate) fn from_byte(b: u8) -> Result<Self> {
        match b {
            0 => Ok(SimilarityKind::Cosine),
            1 => Ok(SimilarityKind::DotProduct),
            2 => Ok(SimilarityKind::Euclidean),
            3 => Err(HelixiteError::Codec(
                "custom similarity cannot be loaded from persisted index".into(),
            )),
            other => Err(HelixiteError::Codec(format!(
                "unknown similarity kind: {other}"
            ))),
        }
    }
}
