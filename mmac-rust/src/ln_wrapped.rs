#![allow(dead_code)]
use derive_more::*;
use ndarray::{Array, ArrayBase, Data, Dimension, Zip};
use paste::paste;
use timing_shield::{TpBool, TpCondSwap, TpOrd};

#[derive(PartialEq, PartialOrd, Clone, Copy, FromStr, Display, Debug)]
pub struct LnWrapped(f32);

impl LnWrapped {
    pub const ONE: Self = Self(0.);
    pub const ZERO: Self = Self(f32::NEG_INFINITY);
    pub const EPS: Self = Self(-69.0775527898);
    pub const NAN: Self = Self(f32::NAN);

    #[inline]
    pub fn is_zero(self) -> bool {
        self.0.is_infinite() & self.0.is_sign_negative()
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    pub fn from_f32_no_ln(f: f32) -> Self {
        Self(f)
    }

    pub fn select_from_4_f32(
        cond0: TpBool,
        cond1: TpBool,
        a11: f32,
        a10: f32,
        a01: f32,
        a00: f32,
    ) -> Self {
        let out = if cond0.expose() {
            if cond1.expose() {
                a11
            } else {
                a10
            }
        } else {
            if cond1.expose() {
                a01
            } else {
                a00
            }
        };
        Self(out.ln())
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
fn lse(a: f32, b: f32) -> f32 {
    (a.exp() + b.exp()).ln()
}

#[inline]
fn lme(a: f32, b: f32) -> f32 {
    (a.exp() - b.exp()).ln()
}

num_op!(lse, Add, add);
num_op!(lme, Sub, sub);
num_op!(std::ops::Add::add, Mul, mul);
num_op!(std::ops::Sub::sub, Div, div);

impl TpOrd for LnWrapped {
    fn tp_lt(&self, rhs: &Self) -> TpBool {
        TpBool::protect(self.0 < rhs.0)
    }

    fn tp_lt_eq(&self, rhs: &Self) -> TpBool {
        TpBool::protect(self.0 <= rhs.0)
    }

    fn tp_gt(&self, rhs: &Self) -> TpBool {
        TpBool::protect(self.0 > rhs.0)
    }

    fn tp_gt_eq(&self, rhs: &Self) -> TpBool {
        TpBool::protect(self.0 >= rhs.0)
    }
}

impl TpCondSwap for LnWrapped {
    fn tp_cond_swap(cond: TpBool, a: &mut Self, b: &mut Self) {
        if cond.expose() {
            std::mem::swap(a, b);
        }
    }
}

impl From<u16> for LnWrapped {
    fn from(u: u16) -> Self {
        Self(f32::from(u).ln())
    }
}

impl From<f32> for LnWrapped {
    fn from(f: f32) -> Self {
        Self(f.ln())
    }
}

impl Into<f32> for LnWrapped {
    fn into(self) -> f32 {
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
        Self(iter.fold(0f32, |acc, x| acc + x.0.exp()).ln())
    }
}

impl<'a> std::iter::Sum<&'a LnWrapped> for LnWrapped {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a LnWrapped>,
    {
        Self(iter.fold(0f32, |acc, x| acc + x.0.exp()).ln())
    }
}
