use super::FixedInner;
use ndarray::{Array, ArrayBase, Data, Dimension, Zip};
use paste::paste;
use std::ops::{Add, Sub};
use timing_shield::{TpBool, TpCondSwap, TpEq, TpOrd};
use typenum::marker_traits::Unsigned;

#[derive(Clone, Copy)]
pub struct LnFixed<F: Unsigned>(FixedInner<F>);

impl<F: Unsigned> LnFixed<F> {
    //TODO remove this
    pub const ONE: Self = Self(FixedInner::ZERO);
    pub const NAN: Self = Self(FixedInner::NAN);

    pub fn leaky_from_f64(f: f64) -> Self {
        Self(FixedInner::leaky_from_f64(f.ln()))
    }

    pub fn leaky_from_i64(i: i64) -> Self {
        Self(FixedInner::leaky_from_f64((i as f64).ln()))
    }

    pub fn leaky_into_f64(self) -> f64 {
        if self.leaky_is_nan() {
            return f64::NAN;
        }
        self.0.leaky_into_f64().exp()
    }

    pub fn leaky_is_nan(self) -> bool {
        self.0.tp_eq(&Self::NAN.0).expose()
    }

    pub fn select_from_4_f64(
        cond0: TpBool,
        cond1: TpBool,
        a11: f64,
        a10: f64,
        a01: f64,
        a00: f64,
    ) -> Self {
        Self(FixedInner::<F>::select_from_4_f64(
            cond0,
            cond1,
            a11.ln(),
            a10.ln(),
            a01.ln(),
            a00.ln(),
        ))
    }
}

impl<F: Unsigned> From<u32> for LnFixed<F> {
    fn from(u: u32) -> Self {
        Self::leaky_from_i64(u as i64)
    }
}

impl<F: Unsigned> From<f64> for LnFixed<F> {
    fn from(f: f64) -> Self {
        Self::leaky_from_f64(f)
    }
}

impl<F: Unsigned> Into<f64> for LnFixed<F> {
    fn into(self) -> f64 {
        self.leaky_into_f64()
    }
}

macro_rules! impl_arith {
    ($op_name: expr, $op: expr,  $trt: path) => {
        paste! {
            impl<F: Unsigned> std::ops::$trt for LnFixed<F> {
                type Output = Self;
                #[inline]
                fn $op_name(self, other: Self) -> Self {
                    Self(self.0.$op(other.0))
                }
            }

            impl<F: Unsigned> std::ops::[<$trt Assign>] for LnFixed<F> {
                #[inline]
                fn [<$op_name _assign>](&mut self, other: Self) {
                    self.0 = self.0.$op(other.0);
                }
            }

            impl<'a, S, D, F> std::ops::$trt<&'a ArrayBase<S, D>> for LnFixed<F>
                where
                    S: Data<Elem = LnFixed<F>>,
                    D: Dimension,
                    F: Unsigned,
                    {
                        type Output = Array<LnFixed<F>, D>;
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

impl_arith! {add, lse, Add}
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
            impl<F: Unsigned> TpEq<$in> for LnFixed<F> {
                [<impl_ord $ext>]! {tp_eq, $in}
                [<impl_ord $ext>]! {tp_not_eq, $in}
            }

            impl<F: Unsigned> TpOrd<$in> for LnFixed<F> {
                [<impl_ord $ext>]! {tp_lt, $in}
                [<impl_ord $ext>]! {tp_lt_eq, $in}
                [<impl_ord $ext>]! {tp_gt, $in}
                [<impl_ord $ext>]! {tp_gt_eq, $in}
            }
        }
    };
}

impl_all_ord! { Self, _none }

impl<F: Unsigned> TpCondSwap for LnFixed<F> {
    #[inline]
    fn tp_cond_swap(condition: TpBool, a: &mut Self, b: &mut Self) {
        FixedInner::<F>::tp_cond_swap(condition, &mut a.0, &mut b.0);
    }
}

impl<F: Unsigned> std::fmt::Display for LnFixed<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let i: f64 = (*self).leaky_into_f64();
        i.fmt(f)
    }
}

impl<F: Unsigned> num_traits::identities::Zero for LnFixed<F> {
    fn zero() -> Self {
        panic!("This should never be called!");
    }

    fn is_zero(&self) -> bool {
        false
    }
}

impl<F: Unsigned> num_traits::identities::One for LnFixed<F> {
    fn one() -> Self {
        Self::ONE
    }
}

impl<F: Unsigned + 'static> ndarray::ScalarOperand for LnFixed<F> {}

impl<F: Unsigned> std::iter::Sum<LnFixed<F>> for LnFixed<F> {
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

impl<'a, F: Unsigned + 'static> std::iter::Sum<&'a LnFixed<F>> for LnFixed<F> {
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

    type F = LnFixed<typenum::U20>;

    #[test]
    fn conversion_test() {
        let reference = 123.123456789123456789f64;
        let a = F::leaky_from_f64(reference);
        let res = a.leaky_into_f64();
        assert!((reference - res).abs() < 1e-3);
    }

    #[test]
    fn add_test() {
        let a = 2f64.exp();
        let b = 1f64.exp();
        let reference = a + b;
        let res = (F::leaky_from_f64(a) + F::leaky_from_f64(b)).leaky_into_f64();
        assert!((reference - res).abs() < 1e-2);
    }

    #[test]
    fn sub_test() {
        let a = 3f64.exp();
        let b = 1f64.exp();
        let reference = a - b;
        let res = (F::leaky_from_f64(a) - F::leaky_from_f64(b)).leaky_into_f64();
        assert!((reference - res).abs() < 1e-2);
    }

    macro_rules! select_from_4_f64_test_single {
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
            let res = F::select_from_4_f64(
                TpBool::protect($cond1),
                TpBool::protect($cond2),
                1.,
                2.,
                3.,
                4.,
            )
            .leaky_into_f64();
            assert!((reference - res).abs() < 1e-5);
        };
    }

    #[test]
    fn select_from_4_f64_test() {
        select_from_4_f64_test_single! {false, false};
        select_from_4_f64_test_single! {true, false};
        select_from_4_f64_test_single! {false, true};
        select_from_4_f64_test_single! {true, true};
    }
}
