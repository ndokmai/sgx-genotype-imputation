use super::*;
use paste::paste;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::mem::transmute;
use timing_shield::{TpBool, TpCondSwap, TpEq, TpI32, TpOrd};

macro_rules! new_self {
    ($inner: expr) => {
        Self { inner: $inner }
    };
}

// TODO This is a quick and dirty way to const initialize TpI32. It should be fixed by implementing
// our own version of TpI32
macro_rules! new_self_raw {
    ($inner: expr) => {
        new_self!(unsafe { transmute($inner) })
    };
}

/// Fixed in regular (no log) space. For internal use only.
#[derive(Clone, Copy)]
pub struct TpFixed32<const F: usize> {
    inner: TpI32,
}

impl<const F: usize> TpFixed32<F> {
    pub const ZERO: Self = new_self_raw!(0i32);
    pub const NAN: Self = new_self_raw!(i32::MAX);

    pub const fn protect_f32(f: f32) -> Self {
        new_self_raw!((f * (1 << F) as f32) as i32)
    }

    pub const fn protect_i32(i: i32) -> Self {
        new_self_raw!(i << F)
    }

    pub fn expose_into_f32(self) -> f32 {
        self.inner.expose() as f32 / (F as f32).exp2()
    }

    pub fn into_inner(self) -> TpI32 {
        self.inner
    }

    pub fn select_from_4_f32(
        cond0: TpBool,
        cond1: TpBool,
        a11: f32,
        a10: f32,
        a01: f32,
        a00: f32,
    ) -> Self {
        let a11 = Self::protect_f32(a11).inner;
        let a10 = Self::protect_f32(a10).inner;
        let a01 = Self::protect_f32(a01).inner;
        let a00 = Self::protect_f32(a00).inner;
        let a0 = cond0.select(a10, a00);
        let a1 = cond0.select(a11, a01);
        new_self!(cond1.select(a1, a0))
    }

    /// lse(a, b) = ln(exp(a) + exp(b))
    pub fn lse(self, other: Self) -> Self {
        let a: TpFixed64<F> = self.into();
        let b: TpFixed64<F> = other.into();
        a.lse(b).into()
    }

    /// lde(a, b) = ln(exp(a) - exp(b))
    pub fn lde(self, other: Self) -> Self {
        let a: TpFixed64<F> = self.into();
        let b: TpFixed64<F> = other.into();
        a.lde(b).into()
    }
}

impl<const F: usize> From<TpFixed64<F>> for TpFixed32<F> {
    fn from(v: TpFixed64<F>) -> Self {
        new_self!(v.into_inner().as_i32())
    }
}

macro_rules! impl_arith {
    ($op: ident, $trait: ident) => {
        paste! {
            impl<const F: usize> std::ops::$trait for TpFixed32<F> {
                type Output = Self;
                #[inline]
                fn $op(self, rhs: Self) -> Self::Output {
                    new_self!(self.inner.$op(rhs.inner))
                }
            }
            impl<const F: usize> std::ops::[<$trait Assign>] for TpFixed32<F> {
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
            impl<const F: usize> std::ops::$trait<$rhs> for TpFixed32<F> {
                type Output = Self;
                #[inline]
                fn $op(self, rhs: $rhs) -> Self::Output {
                    new_self!(self.inner.$op(rhs))
                }
            }
            impl<const F: usize> std::ops::[<$trait Assign>]<$rhs> for TpFixed32<F> {
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

impl<const F: usize> std::ops::Neg for TpFixed32<F> {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self::Output {
        new_self!(self.inner.neg())
    }
}

impl<const F: usize> std::ops::BitAnd<TpI32> for TpFixed32<F> {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: TpI32) -> Self::Output {
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
            impl<const F: usize> TpEq<$in> for TpFixed32<F> {
                [<impl_ord $ext>]! {tp_eq, $in}
                [<impl_ord $ext>]! {tp_not_eq, $in}
            }

            impl<const F: usize> TpOrd<$in> for TpFixed32<F> {
                [<impl_ord $ext>]! {tp_lt, $in}
                [<impl_ord $ext>]! {tp_lt_eq, $in}
                [<impl_ord $ext>]! {tp_gt, $in}
                [<impl_ord $ext>]! {tp_gt_eq, $in}
            }
        }
    };
}

impl_all_ord! { i32, _rhs }
impl_all_ord! { Self, _none }

impl<const F: usize> TpCondSwap for TpFixed32<F> {
    #[inline]
    fn tp_cond_swap(condition: TpBool, a: &mut Self, b: &mut Self) {
        TpI32::tp_cond_swap(condition, &mut a.inner, &mut b.inner);
    }
}

impl<const F: usize> Serialize for TpFixed32<F> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u32(unsafe { transmute(self.inner) })
    }
}

pub struct TpFixed32Visitor<const F: usize>;

impl<'de, const F: usize> serde::de::Visitor<'de> for TpFixed32Visitor<F> {
    type Value = TpFixed32<F>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Error serializing TpFixed32")
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(TpFixed32::<F> {
            inner: unsafe { transmute(value) },
        })
    }
}

impl<'de, const F: usize> Deserialize<'de> for TpFixed32<F> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_u32(TpFixed32Visitor::<F>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    type F = TpFixed32<20>;

    #[test]
    fn conversion_test() {
        let reference = 123.123456789123456789f32;
        let a = F::protect_f32(reference);
        let res = a.expose_into_f32();
        assert!((reference - res).abs() < 1e-6);
    }

    #[test]
    fn lse_test() {
        let a = 11f32;
        let b = 9f32;
        let reference = (a.exp() + b.exp()).ln();
        let res = F::protect_f32(a)
            .lse(F::protect_f32(b))
            .expose_into_f32();
        assert!((reference - res).abs() < 1e-3);
    }

    #[test]
    fn lde_test() {
        let a = 11f32;
        let b = 9f32;
        let reference = (a.exp() - b.exp()).ln();
        let res = F::protect_f32(a)
            .lde(F::protect_f32(b))
            .expose_into_f32();
        assert!((reference - res).abs() < 1e-3);
    }

    macro_rules! select_from_4_f32_test_single {
        ($cond1: expr, $cond2: expr) => {
            let reference = if $cond1 {
                if $cond2 {
                    1.
                } else {
                    2.
                }
            } else {
                if $cond2 {
                    3.
                } else {
                    4.
                }
            };
            let res = F::select_from_4_f32(
                TpBool::protect($cond1),
                TpBool::protect($cond2),
                1.,
                2.,
                3.,
                4.,
            )
            .expose_into_f32();
            assert!((reference - res).abs() < f32::EPSILON);
        };
    }

    #[test]
    fn select_from_4_f32_test() {
        select_from_4_f32_test_single! {false, false};
        select_from_4_f32_test_single! {true, false};
        select_from_4_f32_test_single! {false, true};
        select_from_4_f32_test_single! {true, true};
    }

    #[test]
    fn serialize_test() {
        let reference = 123.123456789123456789f32;
        let a = F::protect_f32(reference);
        let encoded: Vec<u8> = bincode::serialize(&a).unwrap();
        let decoded: F = bincode::deserialize(&encoded[..]).unwrap();
        let res = decoded.expose_into_f32();
        assert!((reference - res).abs() < 1e-6);
    }
}
