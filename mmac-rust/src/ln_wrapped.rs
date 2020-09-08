#![allow(dead_code)]
use derive_more::*;
use ndarray::{Array, ArrayBase, Data, Dimension, Zip};
use paste::paste;

#[derive(PartialEq, PartialOrd, Clone, Copy, FromStr, Display, Debug)]
pub struct LnWrapped(f64);

impl LnWrapped {
    pub const ONE: Self = Self(0.);
    pub const ZERO: Self = Self(f64::NEG_INFINITY);

    #[inline]
    pub fn safe_add(self, other: Self) -> Self {
        self + other
    }

    #[inline]
    pub fn safe_sub(self, other: Self) -> Self {
        self - other
    }

    #[inline]
    pub fn safe_mul(self, other: Self) -> Self {
        self * other
    }

    #[inline]
    pub fn safe_div(self, other: Self) -> Self {
        self / other
    }

    #[inline]
    pub fn is_zero(self) -> bool {
        self.0.is_infinite() & self.0.is_sign_negative()
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    pub fn from_f64_no_ln(f: f64) -> Self {
        Self(f)
    }
}

macro_rules! num_op {
    ($op: expr, $trt: path, $op_name: expr) => {
        paste! {
            impl std::ops::$trt for LnWrapped {
                type Output = Self;
                #[inline]
                fn $op_name(self, other: Self) -> Self {
                    Self($op(self.0, other.0))
                }
            }

            impl std::ops::[<$trt Assign>] for LnWrapped {
                #[inline]
                fn [<$op_name _assign>](&mut self, other: Self) {
                    self.0 = $op(self.0, other.0);
                }
            }

            impl<'a, S, D> std::ops::$trt<&'a ArrayBase<S, D>> for LnWrapped
                where
                    S: Data<Elem = LnWrapped>,
                    D: Dimension,
                    {
                        type Output = Array<LnWrapped, D>;
                        fn $op_name(self, rhs: &ArrayBase<S, D>) -> Self::Output {
                            let mut out = Self::Output::zeros(rhs.dim());
                            Zip::from(&mut out)
                                .and(rhs)
                                .apply(|o, &r| { o.0 = $op(self.0, r.0) });
                            out
                        }
                    }
        }
    };
}

#[inline]
fn lse(a: f64, b: f64) -> f64 {
    (a.exp() + b.exp()).ln()
}

#[inline]
fn lme(a: f64, b: f64) -> f64 {
    (a.exp() - b.exp()).ln()
}

num_op!(lse, Add, add);
num_op!(lme, Sub, sub);
num_op!(std::ops::Add::add, Mul, mul);
num_op!(std::ops::Sub::sub, Div, div);

impl From<u32> for LnWrapped {
    fn from(u: u32) -> Self {
        Self(f64::from(u).ln())
    }
}

impl From<f64> for LnWrapped {
    fn from(f: f64) -> Self {
        Self(f.ln())
    }
}

impl Into<f64> for LnWrapped {
    fn into(self) -> f64 {
        self.0.exp()
    }
}

impl num_traits::identities::Zero for LnWrapped {
    fn zero() -> Self {
        Self::ZERO
    }

    fn is_zero(&self) -> bool {
        Self::is_zero(*self)
    }
}

impl num_traits::identities::One for LnWrapped {
    fn one() -> Self {
        Self::ONE
    }
}

impl ndarray::ScalarOperand for LnWrapped {}

impl std::iter::Sum<LnWrapped> for LnWrapped {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = LnWrapped>,
    {
        Self(iter.fold(0f64, |acc, x| acc + x.0.exp()).ln())
    }
}

impl<'a> std::iter::Sum<&'a LnWrapped> for LnWrapped {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a LnWrapped>,
    {
        Self(iter.fold(0f64, |acc, x| acc + x.0.exp()).ln())
    }
}