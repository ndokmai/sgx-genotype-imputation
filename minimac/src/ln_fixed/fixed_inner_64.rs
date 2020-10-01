use super::*;
use paste::paste;
use std::marker::PhantomData;
use std::mem::transmute;
use timing_shield::{TpBool, TpCondSwap, TpEq, TpI64, TpOrd, TpU32};
use typenum::marker_traits::Unsigned;

macro_rules! new_self {
    ($inner: expr) => {
        Self {
            inner: $inner,
            _phantom: PhantomData,
        }
    };
}

// TODO This is a quick and dirty way to const initialize TpI64. It should be fixed by implementing
// our own version of TpI64
macro_rules! new_self_raw {
    ($inner: expr) => {
        new_self!(unsafe { transmute($inner) })
    };
}

macro_rules! impl_approx {
    ($f_cap: ident, $f: ident) => {
        paste! {
            const [<$f_cap _COEFFS>]: [[Self; $f::N_SEG]; $f::POLY_DEG + 1] =
                Self::[<$f _coeffs_fixed>]();

            const fn [<$f _coeffs_fixed>]() -> [[Self; $f::N_SEG]; $f::POLY_DEG + 1] {
                use $f::*;
                let mut out = [[Self::ZERO; N_SEG]; POLY_DEG + 1];
                let mut i = 0;
                loop {
                    let mut j = 0;
                    loop {
                        out[j][i] = Self::leaky_from_f32(COEFFS[i][j]);
                        j += 1;
                        if j == POLY_DEG + 1 {
                            break;
                        }
                    }
                    i += 1;
                    if i == N_SEG {
                        break;
                    }
                }
                out
            }

            fn $f(self) -> Self {
                use $f::*;
                let f = TpBool::protect(false);
                let t = TpBool::protect(true);
                let mut x = self;
                let mut step = Self::leaky_from_i64(MAX_INPUT >> 1);
                let mut pos_flags = [f; N_SPLIT];
                let mut flag = t;
                for i in 0..N_SPLIT {
                    x = flag.select(x - step, x + step);
                    flag = x.tp_gt_eq(&0);
                    pos_flags[i] = flag;
                    step >>= 1;
                }

                let mut selector = [t; N_SEG];
                for i in 0..N_SEG {
                    let mut mask = 1 << (N_SPLIT - 1);
                    for j in 0..N_SPLIT {
                        let bit = (i & mask) != 0;
                        mask >>= 1;
                        selector[i] &= ! (bit ^ pos_flags[j]);
                    }
                }

                let mut coeffs = [Self::ZERO; POLY_DEG + 1];
                for i in 0..N_SEG {
                    for j in 0..(POLY_DEG + 1) {
                        coeffs[j] += Self::[<$f_cap _COEFFS>][j][i] * selector[i].as_i64();
                    }
                }

                let mut res = coeffs[0] + self * coeffs[1];
                let mut self_pow = self;
                for i in 2..(POLY_DEG + 1) {
                    self_pow *= self;
                    res += self_pow * coeffs[i];
                }
                res
            }
        }
    };
}

/// Fixed in regular (no log) space. For internal use only.
#[derive(Clone, Copy)]
pub struct FixedInner64<F: Unsigned> {
    inner: TpI64,
    _phantom: PhantomData<F>,
}

impl<F: Unsigned> FixedInner64<F> {
    pub const ZERO: Self = new_self_raw!(0i64);
    pub const NAN: Self = new_self_raw!(i64::MAX);

    pub const fn leaky_from_f32(f: f32) -> Self {
        new_self_raw!((f * (1 << F::USIZE) as f32) as i64)
    }

    pub const fn leaky_from_i64(i: i64) -> Self {
        new_self_raw!(i << F::USIZE)
    }

    pub fn leaky_into_f32(self) -> f32 {
        self.inner.expose() as f32 / (F::USIZE as f32).exp2()
    }

    pub fn into_inner(self) -> TpI64 {
        self.inner
    }

    /// lse(a, b) = ln(exp(a) + exp(b))
    pub fn lse(self, other: Self) -> Self {
        let cmp = self.tp_gt_eq(&other);
        let max_val = cmp.select(self, other);
        let diff = cmp.select(self - other, other - self);
        max_val + Self::nls(diff)
    }

    /// lme(a, b) = ln(exp(a) - exp(b))
    pub fn lme(self, other: Self) -> Self {
        let z = self - other;
        self + z.ome().log_lt_one()
    }

    // Piecewise linear approximation to f(a) = ln(1 + exp(-a))
    // Restricted to the positive domain (a >= 0)
    // Approximation level can be adjusted
    impl_approx! {NLS, nls}

    // Piecewise polynomial approximation to f(a) = 1 - exp(-a)
    // Restricted to the positive domain (a >= 0)
    // Approximation level can be adjusted
    impl_approx! {OME, ome}

    /// Approximate log function for domain 0 < a <= 1
    fn log_lt_one(self) -> Self {
        let t = TpBool::protect(true);
        let f = TpBool::protect(false);
        let onehalf = Self::leaky_from_i64(1) >> 1;
        let mut z = self;
        let mut z_scaled = Self::ZERO;

        let mut shift = TpU32::protect(0);
        let mut first_flag = t;
        for _ in 0..(F::USIZE - 1) {
            let bit = z.tp_gt_eq(&onehalf);
            shift += (!bit).as_u32();
            let found = first_flag & bit;
            z_scaled += (z - z_scaled) * found.as_i64();
            first_flag = found.select(f, first_flag);
            z <<= 1;
        }

        // if first_flag is still true then input was zero, just set to smallest non-zero case
        z_scaled = first_flag.select(onehalf, z_scaled);
        shift = first_flag.select(TpU32::protect(F::U32 - 2), shift);

        let zs = z_scaled - Self::leaky_from_i64(1);
        let zs2 = zs * zs;
        let zs3 = zs * zs2;

        let taylor_approx = zs - (zs2 >> 1) + zs3 * Self::leaky_from_f32(1. / 3.);
        let ln2 = Self::leaky_from_f32(0.69314718055994528623);
        taylor_approx - ln2 * shift.as_i64()
    }
}

impl<F: Unsigned> From<FixedInner32<F>> for FixedInner64<F> {
    fn from(v: FixedInner32<F>) -> Self {
        new_self!(v.into_inner().as_i64())
    }
}

macro_rules! impl_arith {
    ($op: ident, $trait: ident) => {
        paste! {
            impl<F: Unsigned> std::ops::$trait for FixedInner64<F> {
                type Output = Self;
                #[inline]
                fn $op(self, rhs: Self) -> Self::Output {
                    new_self!(self.inner.$op(rhs.inner))
                }
            }
            impl<F: Unsigned> std::ops::[<$trait Assign>] for FixedInner64<F> {
                #[inline]
                fn [<$op _assign>](&mut self, rhs: Self) {
                    self.inner.[<$op _assign>](rhs.inner);
                }
            }
        }
    };
}

macro_rules! impl_arith_rhs {
    ($op: ident, $trait: ident, $rhs: ident) => {
        paste! {
            impl<F: Unsigned> std::ops::$trait<$rhs> for FixedInner64<F> {
                type Output = Self;
                #[inline]
                fn $op(self, rhs: $rhs) -> Self::Output {
                    new_self!(self.inner.$op(rhs))
                }
            }
            impl<F: Unsigned> std::ops::[<$trait Assign>]<$rhs> for FixedInner64<F> {
                #[inline]
                fn [<$op _assign>](&mut self, rhs: $rhs) {
                    self.inner.[<$op _assign>](rhs);
                }
            }
        }
    };
}

impl_arith! {add, Add}
impl_arith! {sub, Sub}
impl_arith_rhs! {shr, Shr, u32}
impl_arith_rhs! {shl, Shl, u32}

impl<F: Unsigned> std::ops::Neg for FixedInner64<F> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self::Output {
        new_self!(self.inner.neg())
    }
}

impl<F: Unsigned> std::ops::Mul for FixedInner64<F> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        new_self!((self.inner * rhs.inner) >> F::U32)
    }
}

impl<F: Unsigned> std::ops::MulAssign for FixedInner64<F> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<F: Unsigned> std::ops::Mul<i64> for FixedInner64<F> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: i64) -> Self::Output {
        new_self!(self.inner * rhs)
    }
}

impl<F: Unsigned> std::ops::Mul<TpI64> for FixedInner64<F> {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: TpI64) -> Self::Output {
        new_self!(self.inner * rhs)
    }
}

impl<F: Unsigned> std::ops::MulAssign<TpI64> for FixedInner64<F> {
    #[inline]
    fn mul_assign(&mut self, rhs: TpI64) {
        self.inner *= rhs;
    }
}

impl<F: Unsigned> std::ops::BitAnd<TpI64> for FixedInner64<F> {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: TpI64) -> Self::Output {
        new_self!(self.inner & rhs)
    }
}

macro_rules! impl_ord_none {
    ($op: ident, $in: ident) => {
        #[inline]
        fn $op(&self, rhs: &$in) -> TpBool {
            self.inner.$op(&rhs.inner)
        }
    };
}

macro_rules! impl_ord_rhs {
    ($op: ident, $in: ident) => {
        #[inline]
        fn $op(&self, rhs: &$in) -> TpBool {
            self.inner.$op(rhs)
        }
    };
}

macro_rules! impl_all_ord {
    ($in: ident, $ext: ident) => {
        paste! {
            impl<F: Unsigned> TpEq<$in> for FixedInner64<F> {
                [<impl_ord $ext>]! {tp_eq, $in}
                [<impl_ord $ext>]! {tp_not_eq, $in}
            }

            impl<F: Unsigned> TpOrd<$in> for FixedInner64<F> {
                [<impl_ord $ext>]! {tp_lt, $in}
                [<impl_ord $ext>]! {tp_lt_eq, $in}
                [<impl_ord $ext>]! {tp_gt, $in}
                [<impl_ord $ext>]! {tp_gt_eq, $in}
            }
        }
    };
}

impl_all_ord! { i64, _rhs }
impl_all_ord! { Self, _none }

impl<F: Unsigned> TpCondSwap for FixedInner64<F> {
    #[inline]
    fn tp_cond_swap(condition: TpBool, a: &mut Self, b: &mut Self) {
        TpI64::tp_cond_swap(condition, &mut a.inner, &mut b.inner);
    }
}

// NLS approximation parameters
mod nls {
    pub const N_SPLIT: usize = 4;
    pub const N_SEG: usize = 1 << N_SPLIT;
    pub const MAX_INPUT: i64 = 16;
    pub const POLY_DEG: usize = 2;

    #[rustfmt::skip]
    pub const COEFFS: [[f32; POLY_DEG+1]; N_SEG] = [
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
}

// OME approximation parameters
mod ome {
    pub const N_SPLIT: usize = 4;
    pub const N_SEG: usize = 1 << N_SPLIT;
    pub const MAX_INPUT: i64 = 10;
    pub const POLY_DEG: usize = 2;

    #[rustfmt::skip]
    pub const COEFFS: [[f32; POLY_DEG+1]; N_SEG] = [
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
}

#[cfg(test)]
mod tests {
    use super::*;
    type F = FixedInner64<typenum::U20>;

    #[test]
    fn conversion_test() {
        let reference = 123.123456789123456789f32;
        let a = F::leaky_from_f32(reference);
        let res = a.leaky_into_f32();
        assert!((reference - res).abs() < 1e-6);
    }

    #[test]
    fn nls_test() {
        let a = 0.1234f32;
        let res = F::leaky_from_f32(a).nls().leaky_into_f32();
        let reference = (1. + 1. / a.exp()).ln();
        assert!((reference - res).abs() < 1e-5);
    }

    #[test]
    fn lse_test() {
        let a = 11f32;
        let b = 9f32;
        let reference = (a.exp() + b.exp()).ln();
        let res = F::leaky_from_f32(a)
            .lse(F::leaky_from_f32(b))
            .leaky_into_f32();
        assert!((reference - res).abs() < 1e-3);
    }

    #[test]
    fn lme_test() {
        let a = 11f32;
        let b = 9f32;
        let reference = (a.exp() - b.exp()).ln();
        let res = F::leaky_from_f32(a)
            .lme(F::leaky_from_f32(b))
            .leaky_into_f32();
        assert!((reference - res).abs() < 1e-3);
    }

    #[test]
    fn log_lt_one_test() {
        let a = 0.1234f32;
        let res = F::leaky_from_f32(a).log_lt_one().leaky_into_f32();
        let reference = a.ln();
        assert!((reference - res).abs() < 1e-7);
    }

    #[test]
    fn ome_test() {
        let a = 0.1234f32;
        let res = F::leaky_from_f32(a).ome().leaky_into_f32();
        let reference = 1. - 1. / a.exp();
        assert!((reference - res).abs() < 1e-3);
    }
}
