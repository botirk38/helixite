# Similarity module (agent guidance)

## Purpose

Dispatches similarity computation based on `SimilarityKind`. Only Cosine, DotProduct, and Euclidean survive serialization.

## Safe to edit

- Adding new similarity implementations (new `.rs` files).

## Edit with caution

- `mod.rs` — `SimilarityKind` enum, `to_byte()`/`from_byte()` serialization.
- Adding a new built-in similarity requires: enum variant, `compute()` dispatch, `to_byte()`/`from_byte()` mapping.

## Invariants

- `SimilarityKind::Custom` returns error on `from_byte()` — function pointers cannot be persisted.
- `is_higher_better()` returns `false` for Euclidean — affects HNSW candidate sorting.

## Adding a new similarity

1. Create `new_similarity.rs` with `pub fn compute(a: &[f32], b: &[f32]) -> Result<f32>`.
2. Add variant to `SimilarityKind` in `mod.rs`.
3. Add dispatch in `compute()`.
4. Add `to_byte()`/`from_byte()` mapping.
5. Add `is_higher_better()` case.
6. Add tests.
