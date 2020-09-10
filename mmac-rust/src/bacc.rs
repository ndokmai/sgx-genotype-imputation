use std::ops::{Add, AddAssign};

/// Balanced accumulator
#[derive(Clone)]
pub struct Bacc<T>(Vec<Option<T>>);

impl<T> Bacc<T>
where
    T: std::iter::Sum,
{
    pub fn init() -> Self {
        Self(Vec::new())
    }

    pub fn result(self) -> T {
        self.0
            .into_iter()
            .filter(|v| v.is_some())
            .map(|v| v.unwrap())
            .sum()
    }
}

impl<T> Add<T> for Bacc<T>
where
    T: Add<Output = T> + Copy,
{
    type Output = Self;
    fn add(mut self, val: T) -> Self::Output {
        self.add_assign(val);
        self
    }
}

impl<T> AddAssign<T> for Bacc<T>
where
    T: Add<Output = T> + Copy,
{
    fn add_assign(&mut self, val: T) {
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
