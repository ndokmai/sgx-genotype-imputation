#![allow(dead_code)]
use ndarray::{Array, ArrayBase, Data, Dimension, Zip};
use paste::paste;
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
    pub const ZERO: Self = new_self!(i64::MIN);
    pub const ONE: Self = new_self!(0);
    pub const NAN: Self = new_self!(i64::MAX);
}

impl<F: Unsigned + PartialOrd> LnFixed<F> {
    pub fn max(self, other: Self) -> Self {
        let self_inner = TpI64::protect(self.inner);
        let other_inner = TpI64::protect(other.inner);
        let res = self_inner
            .tp_gt(&other_inner)
            .select(self_inner, other_inner);
        new_self!(res.expose())
    }

    pub fn is_zero(self) -> bool {
        Self::is_zero_inner(self.inner)
    }

    pub fn is_nan(self) -> bool {
        self == Self::NAN
    }

    pub fn from_f64_no_ln(f: f64) -> Self {
        Self::from_f64_inner(f)
    }

    #[inline]
    pub fn safe_add(self, other: Self) -> Self {
        if self.is_zero() {
            return other;
        }
        if other.is_zero() {
            return self;
        }
        debug_assert!(!(self.is_zero() && other.is_zero()));
        self + other
    }

    #[inline]
    pub fn safe_sub(self, other: Self) -> Self {
        if self.is_zero() {
            return Self::ZERO;
        }
        if other.is_zero() {
            return self;
        }
        debug_assert!(!(self.is_zero() && other.is_zero()));
        self - other
    }

    #[inline]
    pub fn safe_mul(self, other: Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::ZERO;
        }
        self * other
    }

    #[inline]
    pub fn safe_div(self, other: Self) -> Self {
        if other.is_zero() {
            return Self::NAN;
        }
        if self.is_zero() {
            return Self::ZERO;
        }
        self / other
    }

    fn is_zero_inner(a: i64) -> bool {
        a == Self::ZERO.inner
    }

    /// lme(a, b) = ln(exp(a) + exp(b))
    /// TODO fix this
    fn lse(a: i64, b: i64) -> i64 {
        debug_assert!(!Self::is_zero_inner(a));
        debug_assert!(!Self::is_zero_inner(b));
        Self::from(Into::<f64>::into(new_self!(a)) + Into::<f64>::into(new_self!(b))).inner
    }

    /// lse(a, b) = ln(exp(a) + exp(b))
    fn _lse(a: i64, b: i64) -> i64 {
        debug_assert!(!Self::is_zero_inner(a));
        debug_assert!(!Self::is_zero_inner(b));
        let flag = (a >= b) as i64;
        let max_val = a * flag + b * (1 - flag);
        let diff = match max_val.checked_mul(2) {
            Some(diff) => diff - a - b,
            None => return 0,
        };
        max_val + Self::nls(diff)
    }

    /// lme(a, b) = ln(exp(a) - exp(b))
    /// TODO fix this
    fn lme(a: i64, b: i64) -> i64 {
        debug_assert!(!Self::is_zero_inner(a));
        debug_assert!(!Self::is_zero_inner(b));
        Self::from(Into::<f64>::into(new_self!(a)) - Into::<f64>::into(new_self!(b))).inner
    }

    /// Piecewise linear approximation to f(a) = ln(1 + exp(-a))
    /// Restricted to the positive domain (a >= 0)
    /// Approximation level can be adjusted
    fn nls(a: i64) -> i64 {
        debug_assert!(!Self::is_zero_inner(a));
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
        let mut self_pow = a;
        for &c in coeffs.iter().skip(2) {
            self_pow = match self_pow.checked_mul(a) {
                Some(r) => r >> F::USIZE,
                None => return 0,
            };
            res += (self_pow * c) >> F::USIZE;
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
        let mut out = [[Self::ZERO; NLS_POLY_DEG + 1]; NLS_N_SEG];
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
        if self.is_zero() {
            return 0.;
        }
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
                    debug_assert!(!self.is_zero());
                    debug_assert!(!other.is_zero());
                    debug_assert!(!self.is_nan());
                    debug_assert!(!other.is_nan());
                    new_self!($op(self.inner, other.inner))
                }
            }

            impl<F: Unsigned + PartialOrd> std::ops::[<$trt Assign>] for LnFixed<F> {
                #[inline]
                fn [<$op_name _assign>](&mut self, other: Self) {
                    debug_assert!(!self.is_zero());
                    debug_assert!(!other.is_zero());
                    debug_assert!(!self.is_nan());
                    debug_assert!(!other.is_nan());
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
        Self::ZERO
    }

    fn is_zero(&self) -> bool {
        Self::is_zero(*self)
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
}
