#![allow(warnings)]

use sp_std::{vec, vec::Vec};
use substrate_fixed::types::I32F32;

// Return true when vector sum is zero.
pub fn is_zero(vector: &[I32F32]) -> bool {
    vector.iter().sum::<I32F32>() == I32F32::from_num(0)
}

// Normalizes (sum to 1 except 0) the input vector directly in-place.
pub fn inplace_normalize(x: &mut [I32F32]) {
    let x_sum: I32F32 = x.iter().sum();
    if x_sum == I32F32::from_num(0.0) {
        return;
    }
    for i in x.iter_mut() {
        *i /= x_sum;
    }
}

pub fn u16_proportion_to_fixed(x: u16) -> I32F32 {
    I32F32::from_num(x) / I32F32::from_num(u16::MAX)
}

pub fn fixed_proportion_to_u16(x: I32F32) -> u16 {
    (x * I32F32::from_num(u16::MAX)).to_num()
}

// Return a new sparse matrix with a masked out diagonal of input sparse matrix.
pub fn mask_diag_sparse(sparse_matrix: &[Vec<(u16, I32F32)>]) -> Vec<Vec<(u16, I32F32)>> {
    let n: usize = sparse_matrix.len();
    let mut result: Vec<Vec<(u16, I32F32)>> = vec![vec![]; n];
    for (i, sparse_row) in sparse_matrix.iter().enumerate() {
        for (j, value) in sparse_row.iter() {
            if i != (*j as usize) {
                result[i].push((*j, *value));
            }
        }
    }
    result
}

/// Normalizes (sum to 1 except 0) each row (dim=0) of a sparse matrix in-place.
pub fn inplace_row_normalize_sparse(sparse_matrix: &mut [Vec<(u16, I32F32)>]) {
    for sparse_row in sparse_matrix.iter_mut() {
        let row_sum: I32F32 = sparse_row.iter().map(|(_j, value)| *value).sum();
        if row_sum > I32F32::from_num(0.0) {
            sparse_row.iter_mut().for_each(|(_j, value)| *value /= row_sum);
        }
    }
}

#[cfg(test)]
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
