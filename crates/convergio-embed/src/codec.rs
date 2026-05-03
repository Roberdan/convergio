//! Pure-function helpers for the [`crate::store`] layer:
//! `f32` ↔ blob round-trip and cosine similarity.
//!
//! Kept in its own module so the storage code stays readable and
//! the per-file 300-line cap is respected.

use crate::error::{EmbedError, Result};

/// Pack a slice of `f32` into a tightly-packed little-endian blob.
pub(crate) fn floats_to_blob(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for f in v {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}

/// Unpack a blob back into `f32`s. Returns
/// [`EmbedError::CorruptBlob`] when the length is not a multiple of
/// 4, and [`EmbedError::DimMismatch`] when the byte count does not
/// match the expected dimension.
pub(crate) fn blob_to_floats(blob: &[u8], dim: usize) -> Result<Vec<f32>> {
    if blob.len() % 4 != 0 {
        return Err(EmbedError::CorruptBlob(blob.len()));
    }
    let got = blob.len() / 4;
    if got != dim {
        return Err(EmbedError::DimMismatch { expected: dim, got });
    }
    let mut out = Vec::with_capacity(dim);
    for chunk in blob.chunks_exact(4) {
        out.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
    }
    Ok(out)
}

/// Euclidean (L2) norm of a vector.
pub(crate) fn norm(v: &[f32]) -> f32 {
    let s: f32 = v.iter().map(|x| x * x).sum();
    s.sqrt()
}

/// Cosine similarity. `a_norm_cached` is the precomputed norm of
/// `a`, supplied by callers that compare a single query against
/// many neighbors so the norm is only computed once.
pub(crate) fn cosine(a: &[f32], b: &[f32], a_norm_cached: f32) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let b_norm = norm(b);
    if a_norm_cached == 0.0 || b_norm == 0.0 {
        return 0.0;
    }
    dot / (a_norm_cached * b_norm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob_roundtrip_preserves_floats() {
        let v = vec![1.0_f32, -2.5, 0.0, 1e-6, 1e10, -1e-10];
        let blob = floats_to_blob(&v);
        assert_eq!(blob.len(), v.len() * 4);
        let back = blob_to_floats(&blob, v.len()).expect("decode");
        assert_eq!(back, v);
    }

    #[test]
    fn corrupt_blob_is_rejected() {
        let bad = vec![1u8, 2, 3];
        assert!(matches!(
            blob_to_floats(&bad, 0),
            Err(EmbedError::CorruptBlob(3))
        ));
    }

    #[test]
    fn dim_mismatch_is_rejected() {
        let v = vec![1.0_f32, 2.0];
        let blob = floats_to_blob(&v);
        assert!(matches!(
            blob_to_floats(&blob, 4),
            Err(EmbedError::DimMismatch {
                expected: 4,
                got: 2,
            })
        ));
    }

    #[test]
    fn cosine_of_identical_vectors_is_one() {
        let a = vec![1.0_f32, 2.0, 3.0];
        let s = cosine(&a, &a, norm(&a));
        assert!((s - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_of_orthogonal_vectors_is_zero() {
        let a = vec![1.0_f32, 0.0];
        let b = vec![0.0_f32, 1.0];
        let s = cosine(&a, &b, norm(&a));
        assert!(s.abs() < 1e-6);
    }

    #[test]
    fn cosine_of_anti_parallel_is_minus_one() {
        let a = vec![1.0_f32, 1.0];
        let b = vec![-1.0_f32, -1.0];
        let s = cosine(&a, &b, norm(&a));
        assert!((s + 1.0).abs() < 1e-6);
    }
}
