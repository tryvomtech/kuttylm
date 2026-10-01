//! Finds the parts whose vectors are closest to the question's vector.

/// Indexes and scores of the `k` best rows. `vectors` holds rows of `dim` numbers.
/// All vectors have length 1, so the dot product is the cosine similarity.
pub fn top_k(query: &[f32], vectors: &[f32], dim: usize, k: usize) -> Vec<(usize, f32)> {
    if dim == 0 || query.len() != dim || k == 0 {
        return Vec::new();
    }
    let mut scored: Vec<(usize, f32)> = vectors
        .chunks_exact(dim)
        .enumerate()
        .map(|(i, row)| (i, row.iter().zip(query).map(|(a, b)| a * b).sum()))
        .filter(|(_, s): &(usize, f32)| s.is_finite())
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    scored.truncate(k);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_rows_first() {
        let rows = [1.0, 0.0, 0.0, 1.0, 0.6, 0.8];
        let hits = top_k(&[0.0, 1.0], &rows, 2, 2);
        assert_eq!(hits.iter().map(|h| h.0).collect::<Vec<_>>(), vec![1, 2]);
        assert!((hits[0].1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn wrong_sizes_give_nothing() {
        assert!(top_k(&[1.0], &[1.0, 0.0], 2, 3).is_empty());
        assert!(top_k(&[1.0, 0.0], &[], 2, 3).is_empty());
    }
}
