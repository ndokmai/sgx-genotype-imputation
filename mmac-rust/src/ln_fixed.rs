#![allow(dead_code)]
use ndarray::{Array, ArrayBase, Data, Dimension, Zip};
use paste::paste;
use std::convert::TryInto;
use std::marker::PhantomData;
use timing_shield::{TpI64, TpOrd};
use typenum::marker_traits::Unsigned;

// NLS approximation parameters
const NLS_N_SPLIT: usize = 4;
const NLS_N_SEG: usize = 1 << NLS_N_SPLIT;
const NLS_MAX_INPUT: f64 = 16.;
const NLS_POLY_DEG: usize = 2;
#[rustfmt::skip]
const NLS_COEFFS: [[f64; NLS_POLY_DEG+1]; NLS_N_SEG] = [
    [0.69273948669433593750, -0.49560832977294921875, 0.11664772033691406250],
    [0.64667129516601562500, -0.40890026092529296875, 0.07470607757568359375],
    [0.49358558654785156250, -0.25441551208496093750, 0.03541278839111328125],
    [0.31156539916992187500, -0.13105392456054687500, 0.01443767547607421875],
    [0.17356395721435546875, -0.06097602844238281250, 0.00552368164062500000],
    [0.08940887451171875000, -0.02685832977294921875, 0.00206184387207031250],
    [0.04373455047607421875, -0.01145648956298828125, 0.00076198577880859375],
    [0.02062034606933593750, -0.00478553771972656250, 0.00028038024902343750],
    [0.00945568084716796875, -0.00196933746337890625, 0.00010299682617187500],
    [0.00424098968505859375, -0.00080108642578125000, 0.00003719329833984375],
    [0.00186824798583984375, -0.00032329559326171875, 0.00001335144042968750],
    [0.00081062316894531250, -0.00012969970703125000, 0.00000476837158203125],
    [0.00034713745117187500, -0.00005149841308593750, 0.00000095367431640625],
    [0.00014686584472656250, -0.00002098083496093750, 0.00000000000000000000],
    [0.00006103515625000000, -0.00000858306884765625, 0.00000000000000000000],
    [0.00000000000000000000,  0.00000000000000000000, 0.00000000000000000000]
];

// OME approximation parameters
const OME_N_SPLIT: usize = 4;
const OME_N_SEG: usize = 1 << OME_N_SPLIT;
const OME_MAX_INPUT: f64 = 10.;
const OME_POLY_DEG: usize = 2;
#[rustfmt::skip]
const OME_COEFFS: [[f64; OME_POLY_DEG+1]; OME_N_SEG] = [
    [0.00156021118164062500, 0.96903133392333984375, -0.36837196350097656250],
    [0.06437397003173828125, 0.76515388488769531250, -0.19717502593994140625],
    [0.20199489593505859375, 0.54148197174072265625, -0.10554027557373046875],
    [0.36964511871337890625, 0.36044883728027343750, -0.05649185180664062500],
    [0.53019905090332031250, 0.23073101043701171875, -0.03023815155029296875],
    [0.66502285003662109375, 0.14373302459716796875, -0.01618576049804687500],
    [0.76923084259033203125, 0.08776378631591796875, -0.00866413116455078125],
    [0.84530639648437500000, 0.05277252197265625000, -0.00463771820068359375],
    [0.89857387542724609375, 0.03134918212890625000, -0.00248241424560546875],
    [0.93470382690429687500, 0.01844024658203125000, -0.00132942199707031250],
    [0.95860195159912109375, 0.01075935363769531250, -0.00071144104003906250],
    [0.97409248352050781250, 0.00623416900634765625, -0.00038146972656250000],
    [0.98396682739257812500, 0.00359153747558593750, -0.00020408630371093750],
    [0.99017333984375000000, 0.00205898284912109375, -0.00010967254638671875],
    [0.99402809143066406250, 0.00117492675781250000, -0.00005912780761718750],
    [1.00000000000000000000, 0.00000000000000000000, 0.00000000000000000000]
];

macro_rules! new_self {
    ($inner: expr) => {
        Self {
            inner: $inner,
            _phantom: PhantomData,
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct LnFixed<F: Unsigned> {
    inner: i64,
    _phantom: PhantomData<F>,
}

impl<F: Unsigned> LnFixed<F> {
    //const ZERO: Self = new_self!(i64::MIN);
    pub const ONE: Self = new_self!(0);
    //TODO remove this
    pub const NAN: Self = new_self!(i64::MAX);
}

impl<F: Unsigned + PartialOrd> LnFixed<F> {
    fn new(inner: i64) -> Self {
        new_self!(inner)
    }

    pub fn max(self, other: Self) -> Self {
        let self_inner = TpI64::protect(self.inner);
        let other_inner = TpI64::protect(other.inner);
        let res = self_inner
            .tp_gt(&other_inner)
            .select(self_inner, other_inner);
        new_self!(res.expose())
    }

    //pub fn is_zero(self) -> bool {
    //Self::is_zero_inner(self.inner)
    //}

    pub fn is_nan(self) -> bool {
        self == Self::NAN
    }

    pub fn from_f64_no_ln(f: f64) -> Self {
        Self::from_f64_inner(f)
    }

    //#[inline]
    //pub fn safe_add(self, other: Self) -> Self {
    //if self.is_zero() {
    //return other;
    //}
    //if other.is_zero() {
    //return self;
    //}
    //debug_assert!(!(self.is_zero() && other.is_zero()));
    //self + other
    //}

    //#[inline]
    //pub fn safe_sub(self, other: Self) -> Self {
    //if self.is_zero() {
    //return Self::ZERO;
    //}
    //if other.is_zero() {
    //return self;
    //}
    //debug_assert!(!(self.is_zero() && other.is_zero()));
    //self - other
    //}

    //#[inline]
    //pub fn safe_mul(self, other: Self) -> Self {
    //if self.is_zero() || other.is_zero() {
    //return Self::ZERO;
    //}
    //self * other
    //}

    //#[inline]
    //pub fn safe_div(self, other: Self) -> Self {
    //if other.is_zero() {
    //return Self::NAN;
    //}
    //if self.is_zero() {
    //return Self::ZERO;
    //}
    //self / other
    //}

    /// lse(a, b) = ln(exp(a) + exp(b))
    fn lse(a: i64, b: i64) -> i64 {
        let flag = (a >= b) as i64;
        let max_val = a * flag + b * (1 - flag);
        let diff = match max_val.checked_mul(2) {
            Some(diff) => diff - a - b,
            None => return 0,
        };
        max_val + Self::nls(diff)
    }

    /// lme(a, b) = ln(exp(a) + exp(b))
    /// TODO fix this
    fn _lse(a: i64, b: i64) -> i64 {
        Self::from(Into::<f64>::into(new_self!(a)) + Into::<f64>::into(new_self!(b))).inner
    }

    /// lme(a, b) = ln(exp(a) - exp(b))
    fn lme(a: i64, b: i64) -> i64 {
        let z = a - b;
        a + Self::fp_log_lt_one(Self::ome(z))
    }

    /// lme(a, b) = ln(exp(a) - exp(b))
    /// TODO fix this
    fn _lme(a: i64, b: i64) -> i64 {
        Self::from(Into::<f64>::into(new_self!(a)) - Into::<f64>::into(new_self!(b))).inner
    }

    /// Piecewise linear approximation to f(a) = ln(1 + exp(-a))
    /// Restricted to the positive domain (a >= 0)
    /// Approximation level can be adjusted
    fn nls(a: i64) -> i64 {
        let mut x = a;
        let mut step = Self::from_i64_inner(NLS_MAX_INPUT as i64 / 2).inner;
        let mut pos_flags = [0i64; NLS_N_SPLIT];
        let mut flag = 1i64;
        for pos_flag in pos_flags.iter_mut() {
            x -= step * (2 * flag - 1);
            flag = (x >= 0) as i64;
            *pos_flag = flag;
            step /= 2;
        }

        let mut selector = [0i64; NLS_N_SEG];
        for i in 0..NLS_N_SEG {
            let mut sel = 1i64;
            for j in 0..NLS_N_SPLIT {
                let bit = ((i & (1 << (NLS_N_SPLIT - j - 1))) > 0) as i64;
                sel *= bit * pos_flags[j] + (1 - bit) * (1 - pos_flags[j]);
            }
            selector[i] = sel;
        }

        let mut coeffs = [0i64; NLS_POLY_DEG + 1];
        for i in 0..NLS_N_SEG {
            for j in 0..(NLS_POLY_DEG + 1) {
                coeffs[j] += Self::NLS_COEFFS[i][j].inner * selector[i];
            }
        }

        let mut res = coeffs[0] + ((a * coeffs[1]) >> F::USIZE);
        let mut a_pow = a;
        for &c in coeffs.iter().skip(2) {
            a_pow = match a_pow.checked_mul(a) {
                Some(r) => r >> F::USIZE,
                None => return 0,
            };
            res += (a_pow * c) >> F::USIZE;
        }
        res
    }

    /// Approximate log function for domain 0 < a <= 1
    fn fp_log_lt_one(a: i64) -> i64 {
        let onehalf = 1u64 << (F::USIZE - 1);
        let mut z = a;
        let mut z_scaled = 0u64;

        let mut shift = 0usize;
        let mut first_flag = 1u64;
        for _ in 0..(F::USIZE - 1) {
            let bit = (z >= onehalf as i64) as u64;
            shift += 1 - bit as usize;
            let found = first_flag * bit;
            z_scaled = found * z as u64 + (1 - found) * z_scaled;
            first_flag = ((1 - found as i64) * first_flag as i64).try_into().unwrap();
            z <<= 1;
        }

        // if first_flag is still true then input was zero, just set to smallest non-zero case
        z_scaled = (first_flag * onehalf + (1 - first_flag) * z_scaled)
            .try_into()
            .unwrap();
        shift = (first_flag * (F::U64 - 2) + (1 - first_flag) * shift as u64)
            .try_into()
            .unwrap();

        let zs = z_scaled as i64 - Self::from_i64_inner(1 as i64).inner;
        let zs2 = (zs * zs) >> F::USIZE;
        let zs3 = (zs * zs2) >> F::USIZE;

        let taylor_approx =
            zs - (zs2 >> 1) + ((zs3 * Self::from_f64_inner(1. / 3.).inner) >> F::USIZE);
        let ln2 = Self::from_f64_inner(0.69314718055994528623).inner;
        taylor_approx - shift as i64 * ln2
    }

    /// Piecewise polynomial approximation to f(a) = 1 - exp(-a)
    /// Restricted to the positive domain (a >= 0)
    /// Approximation level can be adjusted
    fn ome(a: i64) -> i64 {
        let mut x = a;
        let mut step = Self::from_f64_inner(OME_MAX_INPUT / 2.).inner;
        let mut pos_flags = [0i64; OME_N_SPLIT];
        let mut flag = 1i64;

        for p in pos_flags.iter_mut() {
            x -= (2 * flag - 1) * step;
            flag = (x >= 0) as i64;
            *p = flag;
            step /= 2;
        }

        let mut selector = [0i64; OME_N_SEG];
        for i in 0..OME_N_SEG {
            let mut sel = 1i64;
            for j in 0..OME_N_SPLIT {
                let bit = ((i & (1 << (OME_N_SPLIT - j - 1))) > 0) as i64;
                sel *= bit * pos_flags[j] + (1 - bit) * (1 - pos_flags[j]);
            }
            selector[i] = sel;
        }

        let mut coeffs = [0i64; OME_POLY_DEG + 1];
        for i in 0..OME_N_SEG {
            for j in 0..(OME_POLY_DEG + 1) {
                coeffs[j] += selector[i] * Self::OME_COEFFS[i][j].inner;
            }
        }

        let mut res = coeffs[0] + ((a * coeffs[1]) >> F::USIZE);
        let mut a_pow = a;
        for &c in coeffs.iter().skip(2) {
            a_pow = match a_pow.checked_mul(a) {
                Some(r) => r >> F::USIZE,
                None => return 0,
            };
            res += (a_pow * c) >> F::USIZE;
        }
        res
    }

    #[inline]
    const fn from_i64_inner(i: i64) -> Self {
        new_self!(i << F::USIZE)
    }

    #[inline]
    const fn from_f64_inner(f: f64) -> Self {
        new_self!((f * (1 << F::USIZE) as f64) as i64)
    }

    const NLS_COEFFS: [[Self; NLS_POLY_DEG + 1]; NLS_N_SEG] = Self::nsl_coeffs_fixed();

    const fn nsl_coeffs_fixed() -> [[Self; NLS_POLY_DEG + 1]; NLS_N_SEG] {
        let mut out = [[new_self!(0); NLS_POLY_DEG + 1]; NLS_N_SEG];
        let mut i = 0;
        loop {
            let mut j = 0;
            loop {
                out[i][j] = Self::from_f64_inner(NLS_COEFFS[i][j]);
                j += 1;
                if j == NLS_POLY_DEG + 1 {
                    break;
                }
            }
            i += 1;
            if i == NLS_N_SEG {
                break;
            }
        }
        out
    }

    const OME_COEFFS: [[Self; OME_POLY_DEG + 1]; OME_N_SEG] = Self::ome_coeffs_fixed();

    const fn ome_coeffs_fixed() -> [[Self; OME_POLY_DEG + 1]; OME_N_SEG] {
        let mut out = [[new_self!(0); OME_POLY_DEG + 1]; OME_N_SEG];
        let mut i = 0;
        loop {
            let mut j = 0;
            loop {
                out[i][j] = Self::from_f64_inner(OME_COEFFS[i][j]);
                j += 1;
                if j == OME_POLY_DEG + 1 {
                    break;
                }
            }
            i += 1;
            if i == OME_N_SEG {
                break;
            }
        }
        out
    }
}

impl<F: Unsigned> From<u32> for LnFixed<F> {
    fn from(u: u32) -> Self {
        Self::from(u as f64)
    }
}

impl<F: Unsigned> From<f64> for LnFixed<F> {
    fn from(f: f64) -> Self {
        new_self!((f.ln() * (1 << F::USIZE) as f64) as i64)
    }
}

impl<F: Unsigned + PartialOrd> Into<f64> for LnFixed<F> {
    fn into(self) -> f64 {
        if self.is_nan() {
            return f64::NAN;
        }
        (self.inner as f64 / (F::USIZE as f64).exp2()).exp()
    }
}

macro_rules! num_op {
    ($op: expr, $trt: path, $op_name: expr) => {
        paste! {
            impl<F: Unsigned + PartialOrd> std::ops::$trt for LnFixed<F> {
                type Output = Self;
                #[inline]
                fn $op_name(self, other: Self) -> Self {
                    new_self!($op(self.inner, other.inner))
                }
            }

            impl<F: Unsigned + PartialOrd> std::ops::[<$trt Assign>] for LnFixed<F> {
                #[inline]
                fn [<$op_name _assign>](&mut self, other: Self) {
                    self.inner = $op(self.inner, other.inner);
                }
            }

            impl<'a, S, D, F> std::ops::$trt<&'a ArrayBase<S, D>> for LnFixed<F>
                where
                    S: Data<Elem = LnFixed<F>>,
                    D: Dimension,
                    F: Unsigned + PartialOrd,
                    {
                        type Output = Array<LnFixed<F>, D>;
                        fn $op_name(self, rhs: &ArrayBase<S, D>) -> Self::Output {
                            let mut out = rhs.to_owned();
                            Zip::from(&mut out)
                                .apply(|o| { o.inner = $op(o.inner, self.inner) });
                            out
                        }
                    }
        }
    };
}

num_op!(Self::lse, Add, add);
num_op!(Self::lme, Sub, sub);
num_op!(std::ops::Add::add, Mul, mul);
num_op!(std::ops::Sub::sub, Div, div);

impl<F: Unsigned + PartialOrd> std::fmt::Display for LnFixed<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let i: f64 = (*self).into();
        i.fmt(f)
    }
}

impl<F: Unsigned + PartialOrd> num_traits::identities::Zero for LnFixed<F> {
    fn zero() -> Self {
        panic!("This should never be called!");
    }

    fn is_zero(&self) -> bool {
        false
    }
}

impl<F: Unsigned + PartialOrd> num_traits::identities::One for LnFixed<F> {
    fn one() -> Self {
        Self::ONE
    }
}

impl<F: Unsigned + 'static> ndarray::ScalarOperand for LnFixed<F> {}

impl<F: Unsigned + PartialOrd> std::iter::Sum<LnFixed<F>> for LnFixed<F> {
    fn sum<I>(mut iter: I) -> Self
    where
        I: Iterator<Item = LnFixed<F>>,
    {
        let mut accu = Vec::new();
        loop {
            match iter.next() {
                Some(first) => match iter.next() {
                    Some(second) => accu.push(first + second),
                    None => {
                        accu.push(first);
                        break;
                    }
                },
                None => break,
            }
        }
        if accu.is_empty() {
            panic!("Sum of empty iterator.");
        }
        if accu.len() == 1 {
            return accu[0];
        }
        accu.into_iter().sum::<Self>()
    }
}

impl<'a, F: Unsigned + PartialOrd + 'static> std::iter::Sum<&'a LnFixed<F>> for LnFixed<F> {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a LnFixed<F>>,
    {
        iter.cloned().sum::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_test() {
        let a = 123.123456789123456789f64;
        let b = LnFixed::<typenum::U20>::from(a);
        let c: f64 = b.into();
        println!("a = {}", a);
        println!("c = {}", c);
        assert!((a - c).abs() < 1e-3);
    }

    #[inline]
    fn into_f64_inner<F: Unsigned>(a: LnFixed<F>) -> f64 {
        a.inner as f64 / (F::USIZE as f64).exp2()
    }

    #[test]
    fn lse_test() {
        type F = LnFixed<typenum::U20>;
        let a = 11f64;
        let b = 9f64;
        let c = (a.exp() + b.exp()).ln();
        let a_fixed = F::from_f64_inner(a);
        let b_fixed = F::from_f64_inner(b);
        let c_fixed = a_fixed + b_fixed;
        let d: f64 = into_f64_inner(c_fixed);
        println!("c = {}", c);
        println!("d = {}", d);
        assert!((c - d).abs() < 1e-3);
    }

    #[test]
    fn lme_test() {
        type F = LnFixed<typenum::U20>;
        let a = 11f64;
        let b = 9f64;
        let c = (a.exp() - b.exp()).ln();
        let a_fixed = F::from_f64_inner(a);
        let b_fixed = F::from_f64_inner(b);
        let c_fixed = a_fixed - b_fixed;
        let d: f64 = into_f64_inner(c_fixed);
        println!("c = {}", c);
        println!("d = {}", d);
        assert!((c - d).abs() < 1e-3);
    }

    #[test]
    fn fp_log_lt_one_test() {
        type F = LnFixed<typenum::U20>;
        let a = 0.1234f64;
        let a_fixed = F::from_f64_inner(a);
        let res = into_f64_inner(F::new(F::fp_log_lt_one(a_fixed.inner)));
        let reference = a.ln();
        assert!((reference - res).abs() < 1e-7);
    }

    #[test]
    fn ome_test() {
        type F = LnFixed<typenum::U20>;
        let a = 0.1234f64;
        let a_fixed = F::from_f64_inner(a);
        let res = into_f64_inner(F::new(F::ome(a_fixed.inner)));
        let reference = 1. - 1. / a.exp();
        assert!((reference - res).abs() < 1e-3);
    }
}
