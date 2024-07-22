use sp_std::{vec, vec::Vec};
use substrate_fixed::types::{I32F32, I64F64};

// Return true when vector sum is zero.
pub fn is_zero(vector: &[I32F32]) -> bool {
    vector.iter().sum::<I32F32>() == I32F32::from_num(0)
}

#[allow(dead_code)]
pub fn inplace_row_normalize_64(x: &mut [Vec<I64F64>]) {
    for row in x {
        let row_sum: I64F64 = row.iter().sum();
        if row_sum > I64F64::from_num(0.0_f64) {
            row.iter_mut().for_each(|x_ij: &mut I64F64| {
                *x_ij = x_ij.checked_div(row_sum).unwrap_or_default();
            });
        }
    }
}

pub fn fixed64_to_u64(x: I64F64) -> u64 {
    x.to_num::<u64>()
}

pub fn vec_fixed64_to_u64(vec: Vec<I64F64>) -> Vec<u64> {
    vec.into_iter().map(fixed64_to_u64).collect()
}

pub fn matmul_64(matrix: &[Vec<I64F64>], vector: &[I64F64]) -> Vec<I64F64> {
    let Some(first_row) = matrix.first() else {
        return vec![];
    };
    let cols = first_row.len();
    if cols == 0 {
        return vec![];
    }
    assert!(matrix.len() == vector.len());
    matrix
        .iter()
        .zip(vector)
        .fold(vec![I64F64::from_num(0.0); cols], |acc, (row, vec_val)| {
            row.iter()
                .zip(acc)
                .map(|(m_val, acc_val)| {
                    // Compute ranks: r_j = SUM(i) w_ij * s_i
                    // Compute trust scores: t_j = SUM(i) w_ij * s_i
                    // result_j = SUM(i) vector_i * matrix_ij
                    acc_val
                        .checked_add(vec_val.checked_mul(*m_val).unwrap_or_default())
                        .unwrap_or(acc_val)
                })
                .collect()
        })
}

// Normalizes (sum to 1 except 0) the input vector directly in-place.
pub fn inplace_normalize(x: &mut [I32F32]) {
    let x_sum: I32F32 = x.iter().sum();
    if x_sum == I32F32::from_num(0.0) {
        return;
    }
    for i in x.iter_mut() {
        *i = i.saturating_div(x_sum);
    }
}

pub fn u16_proportion_to_fixed(x: u16) -> I32F32 {
    I32F32::from_num(x).saturating_div(I32F32::from_num(u16::MAX))
}

pub fn fixed_proportion_to_u16(x: I32F32) -> u16 {
    (x.saturating_mul(I32F32::from_num(u16::MAX))).to_num()
}

// Return a new sparse matrix with a masked out diagonal of input sparse matrix.
pub fn mask_diag_sparse(sparse_matrix: &[Vec<(u16, I32F32)>]) -> Vec<Vec<(u16, I32F32)>> {
    let n: usize = sparse_matrix.len();
    let mut result: Vec<Vec<(u16, I32F32)>> = vec![vec![]; n];
    for (i, sparse_row) in sparse_matrix.iter().enumerate() {
        for (j, value) in sparse_row.iter() {
            if i != (*j as usize) {
                let Some(row) = result.get_mut(i) else {
                    continue;
                };

                row.push((*j, *value));
            }
        }
    }
    result
}

/// Normalizes (sum to 1 except 0) each row (dim=0) of a sparse matrix in-place.
pub fn inplace_row_normalize_sparse(sparse_matrix: &mut [Vec<(u16, I32F32)>]) {
    for sparse_row in sparse_matrix.iter_mut() {
        let row_sum: I32F32 = sparse_row.iter().map(|(_j, value)| *value).sum();
        if row_sum != I32F32::from_num(0) {
            sparse_row
                .iter_mut()
                .for_each(|(_j, value)| *value = value.saturating_div(row_sum));
        }
    }
}

pub fn matmul_sparse(
    sparse_matrix: &[Vec<(u16, I32F32)>],
    vector: &[I32F32],
    columns: u16,
) -> Vec<I32F32> {
    let mut result: Vec<I32F32> = vec![I32F32::from_num(0.0); columns as usize];
    for (i, sparse_row) in sparse_matrix.iter().enumerate() {
        for (j, value) in sparse_row.iter() {
            // Compute ranks: r_j = SUM(i) w_ij * s_i
            // Compute trust scores: t_j = SUM(i) w_ij * s_i
            // result_j = SUM(i) vector_i * matrix_ij
            let Some(target) = result.get_mut(*j as usize) else {
                continue;
            };
            let Some(vector_i) = vector.get(i) else {
                continue;
            };
            *target = target.saturating_add(vector_i.saturating_mul(*value));
        }
    }
    result
}

pub fn weighted_median_col_sparse(
    stake: &[I32F32],
    score: &[Vec<(u16, I32F32)>],
    columns: u16,
    majority: I32F32,
) -> Vec<I32F32> {
    let rows = stake.len();
    let zero: I32F32 = I32F32::from_num(0);
    let mut use_stake: Vec<I32F32> = stake.iter().copied().filter(|&s| s > zero).collect();
    inplace_normalize(&mut use_stake);
    let stake_sum: I32F32 = use_stake.iter().sum();
    let stake_idx: Vec<usize> = (0..use_stake.len()).collect();
    let minority: I32F32 = stake_sum.saturating_sub(majority);
    let mut use_score: Vec<Vec<I32F32>> = vec![vec![zero; use_stake.len()]; columns as usize];
    let mut median: Vec<I32F32> = vec![zero; columns as usize];
    let mut k: usize = 0;
    for r in 0..rows {
        let Some(stake_r) = stake.get(r) else {
            continue;
        };
        let Some(score_r) = score.get(r) else {
            continue;
        };
        if *stake_r <= zero {
            continue;
        }
        for (c, val) in score_r.iter() {
            let Some(use_score_c) = use_score.get_mut(*c as usize) else {
                continue;
            };
            let Some(use_score_c_k) = use_score_c.get_mut(k) else {
                continue;
            };
            *use_score_c_k = *val;
        }
        k = k.saturating_add(1);
    }
    for c in 0..columns as usize {
        let Some(median_c) = median.get_mut(c) else {
            continue;
        };
        let Some(use_score_c) = use_score.get(c) else {
            continue;
        };
        *median_c = weighted_median(
            &use_stake,
            use_score_c,
            &stake_idx,
            minority,
            zero,
            stake_sum,
        );
    }
    median
}

// Stake-weighted median score finding algorithm, based on a mid pivot binary search.
// Normally a random pivot is used, but to ensure full determinism the mid point is chosen instead.
// Assumes relatively random score order for efficiency, typically less than O(nlogn) complexity.
//
// # Args:
// 	* 'stake': ( &Vec<I32F32> ):
//         - stake, assumed to be normalized.
//
// 	* 'score': ( &Vec<I32F32> ):
//         - score for which median is sought, 0 <= score <= 1
//
// 	* 'partition_idx' ( &Vec<usize> ):
// 		- indices as input partition
//
// 	* 'minority' ( I32F32 ):
// 		- minority_ratio = 1 - majority_ratio
//
// 	* 'partition_lo' ( I32F32 ):
// 		- lower edge of stake for partition, where partition is a segment [lo, hi] inside stake
//     integral [0, 1].
//
// 	* 'partition_hi' ( I32F32 ):
// 		- higher edge of stake for partition, where partition is a segment [lo, hi] inside stake
//     integral [0, 1].
//
// # Returns:
//     * 'median': ( I32F32 ):
//         - median via random pivot binary search.
//
pub fn weighted_median(
    stake: &Vec<I32F32>,
    score: &Vec<I32F32>,
    partition_idx: &[usize],
    minority: I32F32,
    partition_lo: I32F32,
    partition_hi: I32F32,
) -> I32F32 {
    let n = partition_idx.len();
    if n == 0 {
        return I32F32::from_num(0);
    }
    if n == 1 {
        let Some(partition_idx_0) = partition_idx.first() else {
            return I32F32::from_num(0);
        };
        let Some(score_idx) = score.get(*partition_idx_0) else {
            return I32F32::from_num(0);
        };
        return *score_idx;
    }
    assert!(stake.len() == score.len());
    let mid_idx: usize = n / 2;
    let Some(partition_idx_mid_idx) = partition_idx.get(mid_idx) else {
        return I32F32::from_num(0);
    };
    let Some(pivot) = score.get(*partition_idx_mid_idx) else {
        return I32F32::from_num(0);
    };
    let mut lo_stake: I32F32 = I32F32::from_num(0);
    let mut hi_stake: I32F32 = I32F32::from_num(0);
    let mut lower: Vec<usize> = vec![];
    let mut upper: Vec<usize> = vec![];
    for &idx in partition_idx.iter() {
        let Some(score_idx) = score.get(idx) else {
            continue;
        };
        let Some(stake_idx) = stake.get(idx) else {
            continue;
        };
        if *score_idx == *pivot {
            continue;
        }
        if *score_idx < *pivot {
            lo_stake = lo_stake.saturating_add(*stake_idx);
            lower.push(idx);
        } else {
            hi_stake = hi_stake.saturating_add(*stake_idx);
            upper.push(idx);
        }
    }
    if (partition_lo.saturating_add(lo_stake) <= minority)
        && (minority < partition_hi.saturating_sub(hi_stake))
    {
        return *pivot;
    } else if (minority < partition_lo.saturating_add(lo_stake)) && !lower.is_empty() {
        return weighted_median(
            stake,
            score,
            &lower,
            minority,
            partition_lo,
            partition_lo.saturating_add(lo_stake),
        );
    } else if (partition_hi.saturating_sub(hi_stake) <= minority) && !upper.is_empty() {
        return weighted_median(
            stake,
            score,
            &upper,
            minority,
            partition_hi.saturating_sub(hi_stake),
            partition_hi,
        );
    }
    *pivot
}

// Sum across each row (dim=0) of a sparse matrix.
pub fn row_sum_sparse(sparse_matrix: &[Vec<(u16, I32F32)>]) -> Vec<I32F32> {
    let rows = sparse_matrix.len();
    let mut result: Vec<I32F32> = vec![I32F32::from_num(0); rows];
    for (i, sparse_row) in sparse_matrix.iter().enumerate() {
        for (_j, value) in sparse_row.iter() {
            let Some(result_i) = result.get_mut(i) else {
                continue;
            };

            *result_i = result_i.saturating_add(*value);
        }
    }
    result
}

// Return sparse matrix with values above column threshold set to threshold value.
pub fn col_clip_sparse(
    sparse_matrix: &[Vec<(u16, I32F32)>],
    col_threshold: &[I32F32],
) -> Vec<Vec<(u16, I32F32)>> {
    let mut result: Vec<Vec<(u16, I32F32)>> = vec![vec![]; sparse_matrix.len()];
    for (i, sparse_row) in sparse_matrix.iter().enumerate() {
        for (j, value) in sparse_row.iter() {
            let Some(col_threshold_j) = col_threshold.get(*j as usize) else {
                continue;
            };
            let Some(result_i) = result.get_mut(i) else {
                continue;
            };

            if *col_threshold_j < *value {
                result_i.push((*j, *col_threshold_j));
            } else {
                result_i.push((*j, *value));
            }
        }
    }
    result
}

pub fn mask_rows_sparse(
    mask: &[bool],
    sparse_matrix: &[Vec<(u16, I32F32)>],
) -> Vec<Vec<(u16, I32F32)>> {
    if mask.is_empty() {
        return sparse_matrix.to_vec();
    }

    let n: usize = sparse_matrix.len();
    assert_eq!(n, mask.len());

    let mut result: Vec<Vec<(u16, I32F32)>> = vec![vec![]; n];
    for (i, sparse_row) in sparse_matrix.iter().enumerate() {
        let Some(mask_i) = mask.get(i) else {
            continue;
        };
        let Some(result_i) = result.get_mut(i) else {
            continue;
        };
        if !mask_i {
            result_i.clone_from(sparse_row);
        }
    }

    result
}

pub fn vec_mask_sparse_matrix(
    sparse_matrix: &[Vec<(u16, I32F32)>],
    first_vector: &[u64],
    second_vector: &[u64],
    mask_fn: impl Fn(u64, u64) -> bool,
) -> Option<Vec<Vec<(u16, I32F32)>>> {
    let n: usize = sparse_matrix.len();
    let mut result: Vec<Vec<(u16, I32F32)>> = vec![vec![]; n];

    for (i, sparse_row) in sparse_matrix.iter().enumerate() {
        for (j, value) in sparse_row.iter() {
            if !mask_fn(*first_vector.get(i)?, *second_vector.get(*j as usize)?) {
                let Some(result_i) = result.get_mut(i) else {
                    continue;
                };

                result_i.push((*j, *value));
            }
        }
    }

    Some(result)
}

pub fn inplace_mask_vector(mask: &[bool], vector: &mut [I32F32]) {
    if mask.is_empty() {
        return;
    }

    assert_eq!(mask.len(), vector.len());
    let zero: I32F32 = I32F32::from_num(0.0);

    vector
        .iter_mut()
        .enumerate()
        .filter(|(idx, _)| *mask.get(*idx).unwrap_or(&false))
        .for_each(|(_, v)| *v = zero);
}

pub fn inplace_normalize_64(x: &mut [I64F64]) {
    let x_sum: I64F64 = x.iter().sum();
    if x_sum == I64F64::from_num(0) {
        return;
    }

    for x in x {
        *x = x.saturating_div(x_sum);
    }
}

pub fn vec_fixed64_to_fixed32(vec: Vec<I64F64>) -> Vec<I32F32> {
    vec.into_iter().map(I32F32::from_num).collect()
}

pub fn is_topk(vector: &[I32F32], k: usize) -> Vec<bool> {
    let n: usize = vector.len();
    let mut result: Vec<bool> = vec![true; n];
    if n <= k {
        return result;
    }

    let mut idxs: Vec<usize> = (0..n).collect();
    idxs.sort_by_key(|&idx| *vector.get(idx).unwrap_or(&I32F32::from_num(0))); // ascending stable sort
    let Some(idxs_n_sub_k) = idxs.get(..(n.saturating_sub(k))) else {
        return result;
    };
    for idx in idxs_n_sub_k {
        let Some(result_idx) = result.get_mut(*idx) else {
            continue;
        };
        *result_idx = false;
    }

    result
}

pub fn inplace_col_normalize_sparse(sparse_matrix: &mut [Vec<(u16, I32F32)>], columns: u16) {
    let mut col_sum: Vec<I32F32> = vec![I32F32::from_num(0.0); columns as usize]; // assume square matrix, rows=cols

    for sparse_row in sparse_matrix.iter() {
        for (j, value) in sparse_row {
            let Some(col_sum_j) = col_sum.get_mut(*j as usize) else {
                continue;
            };
            *col_sum_j = col_sum_j.saturating_add(*value);
        }
    }

    for sparse_row in sparse_matrix {
        for (j, value) in sparse_row {
            let Some(col_sum_j) = col_sum.get(*j as usize) else {
                continue;
            };
            if *col_sum_j == I32F32::from_num(0.) {
                continue;
            }
            *value = value.saturating_div(*col_sum_j);
        }
    }
}

pub fn row_hadamard_sparse(
    sparse_matrix: &[Vec<(u16, I32F32)>],
    vector: &[I32F32],
) -> Vec<Vec<(u16, I32F32)>> {
    let mut result: Vec<Vec<(u16, I32F32)>> = sparse_matrix.to_vec();
    for (i, sparse_row) in result.iter_mut().enumerate() {
        for (_j, value) in sparse_row {
            let Some(vector_i) = vector.get(i) else {
                continue;
            };
            *value = value.saturating_mul(*vector_i);
        }
    }
    result
}

pub fn inplace_normalize_using_sum(x: &mut [I32F32], x_sum: I32F32) {
    if x_sum == I32F32::from_num(0.) {
        return;
    }

    for x in x {
        *x = x.saturating_div(x_sum);
    }
}

pub fn matmul_transpose_sparse(
    sparse_matrix: &[Vec<(u16, I32F32)>],
    vector: &[I32F32],
) -> Vec<I32F32> {
    let mut result: Vec<I32F32> = vec![I32F32::from_num(0.0); sparse_matrix.len()];
    for (i, sparse_row) in sparse_matrix.iter().enumerate() {
        for (j, value) in sparse_row {
            // Compute dividends: d_j = SUM(i) b_ji * inc_i
            // result_j = SUM(i) vector_i * matrix_ji
            // result_i = SUM(j) vector_j * matrix_ij
            let Some(vector_j) = vector.get(*j as usize) else {
                continue;
            };
            let Some(result_i) = result.get_mut(i) else {
                continue;
            };
            *result_i = result_i.saturating_add(vector_j.saturating_mul(*value))
        }
    }
    result
}

pub fn mat_ema_sparse(
    new: &[Vec<(u16, I32F32)>],
    old: &[Vec<(u16, I32F32)>],
    alpha: I32F32,
) -> Vec<Vec<(u16, I32F32)>> {
    assert!(new.len() == old.len());
    let n = new.len(); // assume square matrix, rows=cols
    let zero: I32F32 = I32F32::from_num(0.0);
    let one_minus_alpha: I32F32 = I32F32::from_num(1.0).saturating_sub(alpha);
    let mut result: Vec<Vec<(u16, I32F32)>> = vec![vec![]; n];
    for i in 0..new.len() {
        let mut row: Vec<I32F32> = vec![zero; n];
        let Some(new_i) = new.get(i) else {
            continue;
        };
        let Some(old_i) = old.get(i) else {
            continue;
        };
        for (j, value) in new_i.iter() {
            let Some(row_j) = row.get_mut(*j as usize) else {
                continue;
            };
            *row_j = row_j.saturating_add(alpha.saturating_mul(*value));
        }
        for (j, value) in old_i.iter() {
            let Some(row_j) = row.get_mut(*j as usize) else {
                continue;
            };
            *row_j = row_j.saturating_add(one_minus_alpha.saturating_mul(*value));
        }
        for (j, value) in row.iter().enumerate() {
            let Some(result_i) = result.get_mut(i) else {
                continue;
            };
            if *value > zero {
                result_i.push((j as u16, *value))
            }
        }
    }
    result
}

/// Max-upscale vector and convert to u16 so max_value = u16::MAX. Assumes non-negative normalized
/// input.
pub fn vec_max_upscale_to_u16(vec: &[I32F32]) -> Vec<u16> {
    let u16_max: I32F32 = I32F32::from_num(u16::MAX);
    let threshold: I32F32 = I32F32::from_num(32768);
    let max_value: Option<&I32F32> = vec.iter().max();
    match max_value {
        Some(val) => {
            if *val == I32F32::from_num(0) {
                return vec
                    .iter()
                    .map(|e: &I32F32| e.saturating_mul(u16_max).to_num::<u16>())
                    .collect();
            }
            if *val > threshold {
                return vec
                    .iter()
                    .map(|e: &I32F32| {
                        e.saturating_mul(u16_max.saturating_div(*val)).round().to_num::<u16>()
                    })
                    .collect();
            }
            return vec
                .iter()
                .map(|e: &I32F32| {
                    e.saturating_mul(u16_max).saturating_div(*val).round().to_num::<u16>()
                })
                .collect();
        }
        None => {
            let sum: I32F32 = vec.iter().sum();
            return vec
                .iter()
                .map(|e: &I32F32| e.saturating_mul(u16_max).saturating_div(sum).to_num::<u16>())
                .collect();
        }
    }
}

pub fn vecdiv(x: &[I32F32], y: &[I32F32]) -> Vec<I32F32> {
    assert_eq!(x.len(), y.len());
    let n = x.len();
    let mut result: Vec<I32F32> = vec![I32F32::from_num(0); n];
    for i in 0..n {
        let Some(y_i) = y.get(i) else {
            continue;
        };
        if *y_i != I32F32::from_num(0.) {
            let Some(result_i) = result.get_mut(i) else {
                continue;
            };
            let Some(x_i) = x.get(i) else {
                continue;
            };
            *result_i = x_i.saturating_div(*y_i);
        }
    }
    result
}

// Max-upscale each column (dim=1) of a sparse matrix in-place.
pub fn inplace_col_max_upscale_sparse(sparse_matrix: &mut [Vec<(u16, I32F32)>], columns: u16) {
    let mut col_max: Vec<I32F32> = vec![I32F32::from_num(0.0); columns as usize]; // assume square matrix, rows=cols
    for sparse_row in sparse_matrix.iter() {
        for (j, value) in sparse_row.iter() {
            let Some(col_max_j) = col_max.get_mut(*j as usize) else {
                continue;
            };
            if *col_max_j < *value {
                *col_max_j = *value;
            }
        }
    }
    for sparse_row in sparse_matrix.iter_mut() {
        for (j, value) in sparse_row.iter_mut() {
            let Some(col_max_j) = col_max.get(*j as usize) else {
                continue;
            };
            if *col_max_j == I32F32::from_num(0.) {
                continue;
            }
            *value = value.saturating_div(*col_max_j);
        }
    }
}

#[cfg(test)]
#[allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]
mod tests {
    use crate::math::*;
    use substrate_fixed::types::{I32F32, I64F64, I96F32};

    macro_rules! fixed_vec {
        () => {
            vec![]
        };
        ($($x:expr),+ $(,)?) => {
            vec![$(I32F32::from_num($x)),+]
        };
    }

    /// Reshape vector to sparse matrix with specified number of input rows, cast f32 to I32F32.
    fn vec_to_sparse_mat_fixed(
        vector: &[f32],
        rows: usize,
        transpose: bool,
    ) -> Vec<Vec<(u16, I32F32)>> {
        assert!(
            vector.len() % rows == 0,
            "Vector of len {:?} cannot reshape to {rows} rows.",
            vector.len()
        );
        let cols: usize = vector.len() / rows;
        let mut mat: Vec<Vec<(u16, I32F32)>> = vec![];
        if transpose {
            for col in 0..cols {
                let mut row_vec: Vec<(u16, I32F32)> = vec![];
                for row in 0..rows {
                    if vector[row * cols + col] > 0. {
                        row_vec.push((row as u16, I32F32::from_num(vector[row * cols + col])));
                    }
                }
                mat.push(row_vec);
            }
        } else {
            for row in 0..rows {
                let mut row_vec: Vec<(u16, I32F32)> = vec![];
                for col in 0..cols {
                    if vector[row * cols + col] > 0. {
                        row_vec.push((col as u16, I32F32::from_num(vector[row * cols + col])));
                    }
                }
                mat.push(row_vec);
            }
        }
        mat
    }

    /// Returns a normalized (sum to 1 except 0) copy of the input vector.
    fn normalize(x: &[I32F32]) -> Vec<I32F32> {
        let x_sum: I32F32 = x.iter().sum();
        if x_sum == I32F32::from_num(0.0) {
            x.to_vec()
        } else {
            x.iter().map(|xi| xi / x_sum).collect()
        }
    }

    fn assert_float_compare(a: I32F32, b: I32F32, epsilon: I32F32) {
        assert!(I32F32::abs(a - b) <= epsilon, "a({a:?}) != b({b:?})");
    }

    fn assert_vec_compare(va: &[I32F32], vb: &[I32F32], epsilon: I32F32) {
        assert!(va.len() == vb.len());
        for (a, b) in va.iter().zip(vb.iter()) {
            assert_float_compare(*a, *b, epsilon);
        }
    }

    fn assert_sparse_mat_compare(
        ma: &[Vec<(u16, I32F32)>],
        mb: &[Vec<(u16, I32F32)>],
        epsilon: I32F32,
    ) {
        assert!(ma.len() == mb.len());
        for row in 0..ma.len() {
            assert!(ma[row].len() == mb[row].len());
            for j in 0..ma[row].len() {
                assert!(ma[row][j].0 == mb[row][j].0); // u16
                assert_float_compare(ma[row][j].1, mb[row][j].1, epsilon) // I32F32
            }
        }
    }

    #[test]
    fn test_math_u64_normalization() {
        let min: u64 = 1;
        let min32: u64 = 4_889_444; // 21_000_000_000_000_000 / 4_294_967_296
        let mid: u64 = 10_500_000_000_000_000;
        let max: u64 = 21_000_000_000_000_000;
        let min_64: I64F64 = I64F64::from_num(min);
        let min32_64: I64F64 = I64F64::from_num(min32);
        let mid_64: I64F64 = I64F64::from_num(mid);
        let max_64: I64F64 = I64F64::from_num(max);
        let max_sum: I64F64 = I64F64::from_num(max);
        let min_frac: I64F64 = min_64 / max_sum;
        assert_eq!(min_frac, I64F64::from_num(0.0000000000000000476));
        let min_frac_32: I32F32 = I32F32::from_num(min_frac);
        assert_eq!(min_frac_32, I32F32::from_num(0));
        let min32_frac: I64F64 = min32_64 / max_sum;
        assert_eq!(min32_frac, I64F64::from_num(0.00000000023283066664));
        let min32_frac_32: I32F32 = I32F32::from_num(min32_frac);
        assert_eq!(min32_frac_32, I32F32::from_num(0.0000000002));
        let half: I64F64 = mid_64 / max_sum;
        assert_eq!(half, I64F64::from_num(0.5));
        let half_32: I32F32 = I32F32::from_num(half);
        assert_eq!(half_32, I32F32::from_num(0.5));
        let one: I64F64 = max_64 / max_sum;
        assert_eq!(one, I64F64::from_num(1));
        let one_32: I32F32 = I32F32::from_num(one);
        assert_eq!(one_32, I32F32::from_num(1));
    }

    #[test]
    fn test_math_to_num() {
        let val: I32F32 = I32F32::from_num(u16::MAX);
        let res: u16 = val.to_num::<u16>();
        assert_eq!(res, u16::MAX);
        let vector: Vec<I32F32> = vec![val; 1000];
        let target: Vec<u16> = vec![u16::MAX; 1000];
        let output: Vec<u16> = vector.iter().map(|e: &I32F32| e.to_num::<u16>()).collect();
        assert_eq!(output, target);
        let output: Vec<u16> = vector.iter().map(|e: &I32F32| (*e).to_num::<u16>()).collect();
        assert_eq!(output, target);
        let val: I32F32 = I32F32::max_value();
        let res: u64 = val.to_num::<u64>();
        let vector: Vec<I32F32> = vec![val; 1000];
        let target: Vec<u64> = vec![res; 1000];
        let output: Vec<u64> = vector.iter().map(|e: &I32F32| e.to_num::<u64>()).collect();
        assert_eq!(output, target);
        let output: Vec<u64> = vector.iter().map(|e: &I32F32| (*e).to_num::<u64>()).collect();
        assert_eq!(output, target);
        let val: I32F32 = I32F32::from_num(0);
        let res: u64 = val.to_num::<u64>();
        let vector: Vec<I32F32> = vec![val; 1000];
        let target: Vec<u64> = vec![res; 1000];
        let output: Vec<u64> = vector.iter().map(|e: &I32F32| e.to_num::<u64>()).collect();
        assert_eq!(output, target);
        let output: Vec<u64> = vector.iter().map(|e: &I32F32| (*e).to_num::<u64>()).collect();
        assert_eq!(output, target);
        let val: I96F32 = I96F32::from_num(u64::MAX);
        let res: u64 = val.to_num::<u64>();
        assert_eq!(res, u64::MAX);
        let vector: Vec<I96F32> = vec![val; 1000];
        let target: Vec<u64> = vec![u64::MAX; 1000];
        let output: Vec<u64> = vector.iter().map(|e: &I96F32| e.to_num::<u64>()).collect();
        assert_eq!(output, target);
        let output: Vec<u64> = vector.iter().map(|e: &I96F32| (*e).to_num::<u64>()).collect();
        assert_eq!(output, target);
    }

    #[test]
    fn test_math_vec_to_sparse_mat_fixed() {
        let vector: Vec<f32> = vec![0., 1., 2., 0., 10., 100.];
        let target: Vec<Vec<(u16, I32F32)>> = vec![
            vec![(1, I32F32::from_num(1.)), (2, I32F32::from_num(2.))],
            vec![(1, I32F32::from_num(10.)), (2, I32F32::from_num(100.))],
        ];
        let mat = vec_to_sparse_mat_fixed(&vector, 2, false);
        assert_sparse_mat_compare(&mat, &target, I32F32::from_num(0));
        let vector: Vec<f32> = vec![0., 0.];
        let target: Vec<Vec<(u16, I32F32)>> = vec![vec![], vec![]];
        let mat = vec_to_sparse_mat_fixed(&vector, 2, false);
        assert_sparse_mat_compare(&mat, &target, I32F32::from_num(0));
        let vector: Vec<f32> = vec![0., 1., 2., 0., 10., 100.];
        let target: Vec<Vec<(u16, I32F32)>> = vec![
            vec![],
            vec![(0, I32F32::from_num(1.)), (1, I32F32::from_num(10.))],
            vec![(0, I32F32::from_num(2.)), (1, I32F32::from_num(100.))],
        ];
        let mat = vec_to_sparse_mat_fixed(&vector, 2, true);
        assert_sparse_mat_compare(&mat, &target, I32F32::from_num(0));
        let vector: Vec<f32> = vec![0., 0.];
        let target: Vec<Vec<(u16, I32F32)>> = vec![vec![]];
        let mat = vec_to_sparse_mat_fixed(&vector, 2, true);
        assert_sparse_mat_compare(&mat, &target, I32F32::from_num(0));
    }

    #[test]
    fn test_math_normalize() {
        let epsilon: I32F32 = I32F32::from_num(0.0001);
        let x: Vec<I32F32> = vec![];
        let y: Vec<I32F32> = normalize(&x);
        assert_vec_compare(&x, &y, epsilon);
        let x: Vec<I32F32> = fixed_vec![1.0, 10.0, 30.0,];
        let y: Vec<I32F32> = normalize(&x);
        assert_vec_compare(
            &y,
            &[
                I32F32::from_num(0.0243902437),
                I32F32::from_num(0.243902439),
                I32F32::from_num(0.7317073171),
            ],
            epsilon,
        );
        assert_float_compare(y.iter().sum(), I32F32::from_num(1.0), epsilon);
        let x: Vec<I32F32> = fixed_vec![-1.0, 10.0, 30.0];
        let y: Vec<I32F32> = normalize(&x);
        assert_vec_compare(
            &y,
            &[
                I32F32::from_num(-0.0256410255),
                I32F32::from_num(0.2564102563),
                I32F32::from_num(0.769230769),
            ],
            epsilon,
        );
        assert_float_compare(y.iter().sum(), I32F32::from_num(1.0), epsilon);
    }

    #[test]
    fn test_math_inplace_normalize() {
        let epsilon: I32F32 = I32F32::from_num(0.0001);
        let mut x1: Vec<I32F32> = fixed_vec![1.0, 10.0, 30.0,];
        inplace_normalize(&mut x1);
        assert_vec_compare(
            &x1,
            &[
                I32F32::from_num(0.0243902437),
                I32F32::from_num(0.243902439),
                I32F32::from_num(0.7317073171),
            ],
            epsilon,
        );
        let mut x2: Vec<I32F32> = fixed_vec![-1.0, 10.0, 30.0,];
        inplace_normalize(&mut x2);
        assert_vec_compare(
            &x2,
            &[
                I32F32::from_num(-0.0256410255),
                I32F32::from_num(0.2564102563),
                I32F32::from_num(0.769230769),
            ],
            epsilon,
        );
    }

    #[test]
    fn test_math_inplace_row_normalize_sparse() {
        let epsilon: I32F32 = I32F32::from_num(0.0001);
        let vector: Vec<f32> = vec![
            0., 1., 0., 2., 0., 3., 4., 0., 1., 0., 2., 0., 3., 0., 1., 0., 0., 2., 0., 3., 4., 0.,
            10., 0., 100., 1000., 0., 10000., 0., 0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.,
            1.,
        ];
        let mut mat = vec_to_sparse_mat_fixed(&vector, 6, false);
        inplace_row_normalize_sparse(&mut mat);
        let target: Vec<f32> = vec![
            0., 0.1, 0., 0.2, 0., 0.3, 0.4, 0., 0.166666, 0., 0.333333, 0., 0.5, 0., 0.1, 0., 0.,
            0.2, 0., 0.3, 0.4, 0., 0.0009, 0., 0.009, 0.09, 0., 0.9, 0., 0., 0., 0., 0., 0., 0.,
            0.142857, 0.142857, 0.142857, 0.142857, 0.142857, 0.142857, 0.142857,
        ];
        assert_sparse_mat_compare(&mat, &vec_to_sparse_mat_fixed(&target, 6, false), epsilon);
        let vector: Vec<f32> = vec![0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.];
        let target: Vec<f32> = vec![0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.];
        let mut mat = vec_to_sparse_mat_fixed(&vector, 3, false);
        inplace_row_normalize_sparse(&mut mat);
        assert_sparse_mat_compare(
            &mat,
            &vec_to_sparse_mat_fixed(&target, 3, false),
            I32F32::from_num(0),
        );
    }

    #[test]
    fn test_math_mask_diag_sparse() {
        let vector: Vec<f32> = vec![1., 2., 3., 4., 5., 6., 7., 8., 9.];
        let target: Vec<f32> = vec![0., 2., 3., 4., 0., 6., 7., 8., 0.];
        let mat = vec_to_sparse_mat_fixed(&vector, 3, false);
        let result = mask_diag_sparse(&mat);
        assert_sparse_mat_compare(
            &result,
            &vec_to_sparse_mat_fixed(&target, 3, false),
            I32F32::from_num(0),
        );
        let vector: Vec<f32> = vec![1., 0., 0., 0., 5., 0., 0., 0., 9.];
        let target: Vec<f32> = vec![0., 0., 0., 0., 0., 0., 0., 0., 0.];
        let mat = vec_to_sparse_mat_fixed(&vector, 3, false);
        let result = mask_diag_sparse(&mat);
        assert_sparse_mat_compare(
            &result,
            &vec_to_sparse_mat_fixed(&target, 3, false),
            I32F32::from_num(0),
        );
        let vector: Vec<f32> = vec![0., 0., 0., 0., 0., 0., 0., 0., 0.];
        let target: Vec<f32> = vec![0., 0., 0., 0., 0., 0., 0., 0., 0.];
        let mat = vec_to_sparse_mat_fixed(&vector, 3, false);
        let result = mask_diag_sparse(&mat);
        assert_sparse_mat_compare(
            &result,
            &vec_to_sparse_mat_fixed(&target, 3, false),
            I32F32::from_num(0),
        );
    }
}
