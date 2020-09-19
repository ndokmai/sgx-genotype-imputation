use crate::Real;
use std::ops::{Add, AddAssign};

/// Balanced accumulator
#[derive(Clone)]
pub struct Bacc(Vec<Option<Real>>);

impl Bacc {
    pub fn init() -> Self {
        Self(Vec::new())
    }

    pub fn result(self) -> Real {
        if self.0.iter().filter(|v| v.is_some()).count() == 0 {
            return Real::EPS;
        }

        self.0
            .into_iter()
            .filter(|v| v.is_some())
            .map(|v| v.unwrap())
            .sum()
    }
}

impl Add<Real> for Bacc {
    type Output = Self;
    fn add(mut self, val: Real) -> Self::Output {
        self.add_assign(val);
        self
    }
}

impl AddAssign<Real> for Bacc {
    fn add_assign(&mut self, val: Real) {
        let mut val = Some(val);
        for slot in self.0.iter_mut() {
            if slot.is_some() {
                val.replace(slot.take().unwrap() + val.unwrap());
            } else {
                slot.replace(val.take().unwrap());
                break;
            }
        }
        if val.is_some() {
            self.0.push(val);
        }
    }
}
