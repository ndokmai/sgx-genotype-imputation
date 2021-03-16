use super::*;
use ndarray::{Array, ArrayBase, Data, Dimension, Zip};
use paste::paste;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::marker::PhantomData;
use std::ops::{Add, Sub};
use timing_shield::{TpBool, TpCondSwap, TpEq, TpOrd};
use typenum::marker_traits::Unsigned;

#[derive(Clone, Copy)]
pub struct TpLnFixed<F: Unsigned>(TpFixedInner32<F>);

impl<F: Unsigned> TpLnFixed<F> {
    //TODO remove this
    pub const ONE: Self = Self(TpFixedInner32::ZERO);
    pub const NAN: Self = Self(TpFixedInner32::NAN);
    pub const EPS: Self = Self(TpFixedInner32::leaky_from_f32(-69.0775527898)); // 1e-30

    pub fn leaky_from_f32(f: f32) -> Self {
        Self(TpFixedInner32::leaky_from_f32(f.ln()))
    }

    pub fn sum_in_place(slice: &mut [Self]) -> Self {
        if slice.is_empty() {
            return Self::EPS;
        } else if slice.len() == 1 {
            return slice[0];
        } else if slice.len() <= 8 {
            return slice[1..].iter().fold(slice[0], |acc, &a| acc + a);
        }
        let first_half_len = (slice.len() + 1) / 2;
        let second_half_len = slice.len() / 2;
        for i in 0..second_half_len {
            slice[i] += slice[i + first_half_len];
        }
        return Self::sum_in_place(&mut slice[..first_half_len]);
    }

    pub fn leaky_from_i32(i: i32) -> Self {
        Self(TpFixedInner32::leaky_from_f32((i as f32).ln()))
    }

    pub fn leaky_into_f32(self) -> f32 {
        if self.leaky_is_nan() {
            return f32::NAN;
        }
        self.0.leaky_into_f32().exp()
    }

    pub fn leaky_is_nan(self) -> bool {
        self.0.tp_eq(&Self::NAN.0).expose()
    }

    pub fn select_from_4_f32(
        cond0: TpBool,
        cond1: TpBool,
        a11: f32,
        a10: f32,
        a01: f32,
        a00: f32,
    ) -> Self {
        Self(TpFixedInner32::<F>::select_from_4_f32(
            cond0,
            cond1,
            a11.ln(),
            a10.ln(),
            a01.ln(),
            a00.ln(),
        ))
    }
}

impl<F: Unsigned> From<u16> for TpLnFixed<F> {
    fn from(u: u16) -> Self {
        Self::leaky_from_i32(u as i32)
    }
}

impl<F: Unsigned> From<f32> for TpLnFixed<F> {
    fn from(f: f32) -> Self {
        Self::leaky_from_f32(f)
    }
}

impl<F: Unsigned> Into<f32> for TpLnFixed<F> {
    fn into(self) -> f32 {
        self.leaky_into_f32()
    }
}

macro_rules! impl_arith {
    ($op_name: expr, $op: expr,  $trt: path) => {
        paste! {
            impl<F: Unsigned> std::ops::$trt for TpLnFixed<F> {
                type Output = Self;
                #[inline]
                fn $op_name(self, other: Self) -> Self {
                    Self(self.0.$op(other.0))
                }
            }

            impl<F: Unsigned> std::ops::[<$trt Assign>] for TpLnFixed<F> {
                #[inline]
                fn [<$op_name _assign>](&mut self, other: Self) {
                    self.0 = self.0.$op(other.0);
                }
            }

            impl<'a, S, D, F> std::ops::$trt<&'a ArrayBase<S, D>> for TpLnFixed<F>
                where
                    S: Data<Elem = TpLnFixed<F>>,
                    D: Dimension,
                    F: Unsigned,
                    {
                        type Output = Array<TpLnFixed<F>, D>;
                        fn $op_name(self, rhs: &ArrayBase<S, D>) -> Self::Output {
                            let mut out = rhs.to_owned();
                            Zip::from(&mut out)
                                .apply(|o| o.0 = o.0.$op(self.0) );
                            out
                        }
                    }
        }
    };
}

impl_arith!(add, lse, Add);
impl_arith!(sub, lme, Sub);
impl_arith!(mul, add, Mul);
impl_arith!(div, sub, Div);

macro_rules! impl_ord_none {
    ($op: ident, $in: ident) => {
        #[inline]
        fn $op(&self, rhs: &$in) -> TpBool {
            self.0.$op(&rhs.0)
        }
    };
}

macro_rules! impl_all_ord {
    ($in: ident, $ext: ident) => {
        paste! {
            impl<F: Unsigned> TpEq<$in> for TpLnFixed<F> {
                [<impl_ord $ext>]! {tp_eq, $in}
                [<impl_ord $ext>]! {tp_not_eq, $in}
            }

            impl<F: Unsigned> TpOrd<$in> for TpLnFixed<F> {
                [<impl_ord $ext>]! {tp_lt, $in}
                [<impl_ord $ext>]! {tp_lt_eq, $in}
                [<impl_ord $ext>]! {tp_gt, $in}
                [<impl_ord $ext>]! {tp_gt_eq, $in}
            }
        }
    };
}

impl_all_ord! { Self, _none }

impl<F: Unsigned> TpCondSwap for TpLnFixed<F> {
    #[inline]
    fn tp_cond_swap(condition: TpBool, a: &mut Self, b: &mut Self) {
        TpFixedInner32::<F>::tp_cond_swap(condition, &mut a.0, &mut b.0);
    }
}

impl<F: Unsigned> std::fmt::Display for TpLnFixed<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let i: f32 = (*self).leaky_into_f32();
        i.fmt(f)
    }
}

impl<F: Unsigned> num_traits::identities::Zero for TpLnFixed<F> {
    fn zero() -> Self {
        panic!("This should never be called!");
    }

    fn is_zero(&self) -> bool {
        false
    }
}

impl<F: Unsigned> num_traits::identities::One for TpLnFixed<F> {
    fn one() -> Self {
        Self::ONE
    }
}

impl<F: Unsigned + 'static> ndarray::ScalarOperand for TpLnFixed<F> {}

impl<F: Unsigned> std::iter::Sum<TpLnFixed<F>> for TpLnFixed<F> {
    fn sum<I>(mut iter: I) -> Self
    where
        I: Iterator<Item = TpLnFixed<F>>,
    {
        let first_pair = match iter.next() {
            Some(first) => match iter.next() {
                Some(second) => first + second,
                None => return first,
            },
            None => return TpLnFixed::EPS,
        };
        let mut accu = Vec::with_capacity(20);
        accu.push(first_pair);
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
        if accu.len() == 1 {
            return accu[0];
        }
        Self::sum_in_place(accu.as_mut_slice())
    }
}

impl<'a, F: Unsigned + 'static> std::iter::Sum<&'a TpLnFixed<F>> for TpLnFixed<F> {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a TpLnFixed<F>>,
    {
        iter.cloned().sum::<Self>()
    }
}

impl<F: Unsigned> Serialize for TpLnFixed<F> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

struct TpLnFixedVisitor<F>(PhantomData<F>);

impl<'de, F: Unsigned> serde::de::Visitor<'de> for TpLnFixedVisitor<F> {
    type Value = TpLnFixed<F>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Error serializing TpFixedInner32")
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        TpFixedInner32Visitor::<F>(PhantomData)
            .visit_u32(value)
            .map(|v| TpLnFixed(v))
    }
}

impl<'de, F: Unsigned> Deserialize<'de> for TpLnFixed<F> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_u32(TpLnFixedVisitor::<F>(PhantomData))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type F = TpLnFixed<typenum::U20>;

    #[test]
    fn conversion_test() {
        let reference = 123.123456789123456789f32;
        let a = F::leaky_from_f32(reference);
        let res = a.leaky_into_f32();
        assert!((reference - res).abs() < 1e-3);
    }

    #[test]
    fn add_test() {
        let a = 2f32.exp();
        let b = 1f32.exp();
        let reference = a + b;
        let res = (F::leaky_from_f32(a) + F::leaky_from_f32(b)).leaky_into_f32();
        assert!((reference - res).abs() < 1e-2);
    }

    #[test]
    fn sub_test() {
        let a = 3f32.exp();
        let b = 1f32.exp();
        let reference = a - b;
        let res = (F::leaky_from_f32(a) - F::leaky_from_f32(b)).leaky_into_f32();
        assert!((reference - res).abs() < 1e-2);
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
            .leaky_into_f32();
            assert!((reference - res).abs() < 1e-5);
        };
    }

    #[test]
    fn select_from_4_f32_test() {
        select_from_4_f32_test_single! {false, false};
        select_from_4_f32_test_single! {true, false};
        select_from_4_f32_test_single! {false, true};
        select_from_4_f32_test_single! {true, true};
    }
}
