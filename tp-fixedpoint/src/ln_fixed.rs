use super::*;
use ndarray::{Array, ArrayBase, Data, Dimension, Zip};
use paste::paste;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::ops::{Add, Sub};
use timing_shield::{TpBool, TpCondSwap, TpEq, TpOrd};

#[derive(Clone, Copy)]
pub struct TpLnFixed<const F: usize>(TpFixed32<F>);

impl<const F: usize> TpLnFixed<F> {
    //TODO remove this
    pub const ONE: Self = Self(TpFixed32::ZERO);
    pub const NAN: Self = Self(TpFixed32::NAN);

    pub fn protect_f32(f: f32) -> Self {
        Self(TpFixed32::protect_f32(f.ln()))
    }

    pub fn sum_in_place(slice: &mut [Self]) -> Self {
        if slice.is_empty() {
            panic!("Cannot sum an empty slice");
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

    pub fn checked_sum_in_place(slice: &mut [Self]) -> Self {
        if slice.is_empty() {
            panic!("Cannot sum an empty slice");
        } else if slice.len() == 1 {
            return slice[0];
        } else if slice.len() <= 8 {
            return slice[1..].iter().fold(slice[0], |acc, &a| {
                acc.is_nan().select(a, a.is_nan().select(acc, acc + a))
            });
        }
        let first_half_len = (slice.len() + 1) / 2;
        let second_half_len = slice.len() / 2;
        for i in 0..second_half_len {
            slice[i] = slice[i + first_half_len].is_nan().select(
                slice[i],
                slice[i].is_nan().select(
                    slice[i + first_half_len],
                    slice[i] + slice[i + first_half_len],
                ),
            );
        }
        return Self::checked_sum_in_place(&mut slice[..first_half_len]);
    }

    pub fn protect_i32(i: i32) -> Self {
        Self(TpFixed32::protect_f32((i as f32).ln()))
    }

    pub fn expose_into_f32(self) -> f32 {
        if self.leaky_is_nan() {
            return f32::NAN;
        }
        self.0.expose_into_f32().exp()
    }

    pub fn leaky_is_nan(self) -> bool {
        self.0.tp_eq(&Self::NAN.0).expose()
    }
    pub fn is_nan(self) -> TpBool {
        self.0.tp_eq(&Self::NAN.0)
    }

    pub fn select_from_4_f32(
        cond0: TpBool,
        cond1: TpBool,
        a11: f32,
        a10: f32,
        a01: f32,
        a00: f32,
    ) -> Self {
        Self(TpFixed32::<F>::select_from_4_f32(
            cond0,
            cond1,
            a11.ln(),
            a10.ln(),
            a01.ln(),
            a00.ln(),
        ))
    }
}

impl<const F: usize> From<u16> for TpLnFixed<F> {
    fn from(u: u16) -> Self {
        Self::protect_i32(u as i32)
    }
}

impl<const F: usize> From<f32> for TpLnFixed<F> {
    fn from(f: f32) -> Self {
        Self::protect_f32(f)
    }
}

impl<const F: usize> Into<f32> for TpLnFixed<F> {
    fn into(self) -> f32 {
        self.expose_into_f32()
    }
}

macro_rules! impl_arith {
    ($op_name: expr, $op: expr,  $trt: path) => {
        paste! {
            impl<const F: usize> std::ops::$trt for TpLnFixed<F> {
                type Output = Self;
                #[inline]
                fn $op_name(self, other: Self) -> Self {
                    Self(self.0.$op(other.0))
                }
            }

            impl<const F: usize> std::ops::[<$trt Assign>] for TpLnFixed<F> {
                #[inline]
                fn [<$op_name _assign>](&mut self, other: Self) {
                    self.0 = self.0.$op(other.0);
                }
            }

            impl<'a, S, D, const F: usize> std::ops::$trt<&'a ArrayBase<S, D>> for TpLnFixed<F>
                where
                    S: Data<Elem = TpLnFixed<F>>,
                    D: Dimension,
                    {
                        type Output = Array<TpLnFixed<F>, D>;
                        fn $op_name(self, rhs: &ArrayBase<S, D>) -> Self::Output {
                            let mut out = rhs.to_owned();
                            Zip::from(&mut out)
                                .for_each(|o| o.0 = o.0.$op(self.0) );
                            out
                        }
                    }
        }
    };
}

impl_arith!(add, lse, Add);
impl_arith!(sub, lde, Sub);
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
            impl<const F: usize> TpEq<$in> for TpLnFixed<F> {
                [<impl_ord $ext>]! {tp_eq, $in}
                [<impl_ord $ext>]! {tp_not_eq, $in}
            }

            impl<const F: usize> TpOrd<$in> for TpLnFixed<F> {
                [<impl_ord $ext>]! {tp_lt, $in}
                [<impl_ord $ext>]! {tp_lt_eq, $in}
                [<impl_ord $ext>]! {tp_gt, $in}
                [<impl_ord $ext>]! {tp_gt_eq, $in}
            }
        }
    };
}

impl_all_ord! { Self, _none }

impl<const F: usize> TpCondSwap for TpLnFixed<F> {
    #[inline]
    fn tp_cond_swap(condition: TpBool, a: &mut Self, b: &mut Self) {
        TpFixed32::<F>::tp_cond_swap(condition, &mut a.0, &mut b.0);
    }
}

impl<const F: usize> std::fmt::Display for TpLnFixed<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let i: f32 = (*self).expose_into_f32();
        i.fmt(f)
    }
}

impl<const F: usize> num_traits::identities::One for TpLnFixed<F> {
    fn one() -> Self {
        Self::ONE
    }
}

impl<const F: usize> ndarray::ScalarOperand for TpLnFixed<F> {}

impl<const F: usize> std::iter::Sum<TpLnFixed<F>> for TpLnFixed<F> {
    fn sum<I>(mut iter: I) -> Self
    where
        I: Iterator<Item = TpLnFixed<F>>,
    {
        let first_pair = match iter.next() {
            Some(first) => match iter.next() {
                Some(second) => first + second,
                None => return first,
            },
            None => panic!("Cannot sum an empty iterator"),
        };
        let mut accu = Vec::new();
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

impl<'a, const F: usize> std::iter::Sum<&'a TpLnFixed<F>> for TpLnFixed<F> {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a TpLnFixed<F>>,
    {
        iter.cloned().sum::<Self>()
    }
}

impl<const F: usize> Serialize for TpLnFixed<F> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

struct TpLnFixedVisitor<const F: usize>;

impl<'de, const F: usize> serde::de::Visitor<'de> for TpLnFixedVisitor<F> {
    type Value = TpLnFixed<F>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("Error serializing TpFixed32")
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        TpFixed32Visitor::<F>.visit_u32(value).map(|v| TpLnFixed(v))
    }
}

impl<'de, const F: usize> Deserialize<'de> for TpLnFixed<F> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_u32(TpLnFixedVisitor::<F>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type F = TpLnFixed<20>;

    #[test]
    fn conversion_test() {
        let reference = 123.123456789123456789f32;
        let a = F::protect_f32(reference);
        let res = a.expose_into_f32();
        assert!((reference - res).abs() < 1e-3);
    }

    #[test]
    fn add_test() {
        let a = 2f32.exp();
        let b = 1f32.exp();
        let reference = a + b;
        let res = (F::protect_f32(a) + F::protect_f32(b)).expose_into_f32();
        assert!((reference - res).abs() < 1e-2);
    }

    #[test]
    fn sub_test() {
        let a = 3f32.exp();
        let b = 1f32.exp();
        let reference = a - b;
        let res = (F::protect_f32(a) - F::protect_f32(b)).expose_into_f32();
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
            .expose_into_f32();
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
