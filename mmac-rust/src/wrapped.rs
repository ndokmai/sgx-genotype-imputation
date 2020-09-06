use derive_more::*;
use ndarray::{Array, ArrayBase, Data, Dimension, Zip};
use paste::paste;

#[derive(PartialEq, PartialOrd, Clone, Copy, From, Into, FromStr, Display, Debug)]
pub struct Wrapped(f64);

impl Wrapped {
    pub const ZERO: Self = Self(0.);
    pub const EPSILON: Self = Self(f64::EPSILON);

    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    pub fn max(self, other: Self) -> Self {
        Self(self.0.max(other.0))
    }

    pub fn is_nan(self) -> bool {
        self.0.is_nan()
    }
}

macro_rules! num_op {
    ($op: tt, $trt: path, $op_name: expr) => {
        paste! {
            impl std::ops::$trt for Wrapped {
                type Output = Self;
                fn $op_name(self, other: Self) -> Self {
                    Self(self.0 $op other.0)
                }
            }

            impl std::ops::[<$trt Assign>] for Wrapped {
                fn [<$op_name _assign>](&mut self, other: Self) {
                    self.0 = self.0 $op other.0;
                }
            }

            impl<'a, S, D> std::ops::$trt<&'a ArrayBase<S, D>> for Wrapped
                where
                    S: Data<Elem = Wrapped>,
                    D: Dimension,
                    {
                        type Output = Array<Wrapped, D>;
                        fn $op_name(self, rhs: &ArrayBase<S, D>) -> Self::Output {
                            let mut out = Self::Output::zeros(rhs.dim());
                            Zip::from(&mut out)
                                .and(rhs)
                                .apply(|o, &r| { o.0 = self.0 $op r.0 });
                            out
                        }
                    }
        }
    };
}

num_op!(+, Add, add);
num_op!(-, Sub, sub);
num_op!(*, Mul, mul);
num_op!(/, Div, div);

impl From<u32> for Wrapped {
    fn from(u: u32) -> Self {
        Self(u.into())
    }
}

impl num_traits::identities::Zero for Wrapped {
    fn zero() -> Self {
        Self(0.)
    }

    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl num_traits::identities::One for Wrapped {
    fn one() -> Self {
        Self(1.)
    }
}

impl ndarray::ScalarOperand for Wrapped {}

impl std::iter::Sum<Wrapped> for Wrapped {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = Wrapped>,
    {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl<'a> std::iter::Sum<&'a Wrapped> for Wrapped {
    fn sum<I>(iter: I) -> Self
    where
        I: Iterator<Item = &'a Wrapped>,
    {
        iter.fold(Self::ZERO, |acc, x| acc + *x)
    }
}
