use bitvec::prelude::{BitSlice, BitStore, BitVec, Lsb0};

#[derive(PartialEq, Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[repr(i8)]
pub enum Symbol {
    Ref = 0,
    Alt = 1,
    Missing = -1,
}

impl From<bool> for Symbol {
    fn from(bit: bool) -> Self {
        match bit {
            false => Self::Ref,
            true => Self::Alt,
        }
    }
}

impl Into<bool> for Symbol {
    fn into(self) -> bool {
        match self {
            Self::Ref => false,
            Self::Alt => true,
            _ => panic!("Invalid symbol"),
        }
    }
}

impl From<(bool, bool)> for Symbol {
    fn from(bits: (bool, bool)) -> Self {
        match bits {
            (true, false) => Self::Ref,
            (true, true) => Self::Alt,
            (false, false) => Self::Missing,
            _ => panic!("Invalid symbol"),
        }
    }
}

impl Into<(bool, bool)> for Symbol {
    fn into(self) -> (bool, bool) {
        match self {
            Self::Ref => (true, false),
            Self::Alt => (true, true),
            Self::Missing => (false, false),
        }
    }
}

impl From<i8> for Symbol {
    fn from(symbol: i8) -> Self {
        match symbol {
            0 => Self::Ref,
            1 => Self::Alt,
            -1 => Self::Missing,
            _ => panic!("Invalid symbol"),
        }
    }
}

impl Into<i8> for Symbol {
    fn into(self) -> i8 {
        self as i8
    }
}

impl std::str::FromStr for Symbol {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<i8>()?.into())
    }
}

#[derive(Clone)]
pub struct SymbolVec<T: BitStore>(BitVec<Lsb0, T>);

impl<T: BitStore> SymbolVec<T> {
    pub fn new() -> Self {
        Self(BitVec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(BitVec::with_capacity(capacity*2))
    }

    pub fn shrink_to(&mut self, new_len: usize) {
        assert!(new_len <= self.0.len());
        self.0.resize(new_len*2, false);
    }

    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit()
    }

    pub fn push(&mut self, s: Symbol) {
        let (first, second) = s.into();
        self.0.push(first);
        self.0.push(second);
    }


    pub fn iter<'a>(&'a self) -> Iter<'a, T> {
        Iter(self.0.as_bitslice().iter())
    }

    pub fn into_inner(self) -> BitVec<Lsb0, T> {
        self.0
    }

    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }

    pub fn as_bitslice(&self) -> &BitSlice<Lsb0, T> {
        self.0.as_bitslice()
    }

}

impl<T: BitStore> From<BitVec<Lsb0, T>> for SymbolVec<T> {
    fn from(inner: BitVec<Lsb0, T>) -> Self {
        Self(inner)
    }
}

pub struct IntoIter<T: 'static + BitStore>(bitvec::vec::IntoIter<Lsb0, T>);

impl<T: BitStore> Iterator for IntoIter<T> {
    type Item = Symbol;
    fn next(&mut self) -> Option<Self::Item> {
        let first = self.0.next()?;
        let second = self.0.next()?;
        Some((first, second).into())
    }
}

impl<T: 'static + BitStore> IntoIterator for SymbolVec<T> {
    type Item = Symbol;
    type IntoIter = IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0.into_iter())
    }
}

pub struct Iter<'a, T: BitStore>(bitvec::slice::Iter<'a, Lsb0, T>);

impl<'a, T: BitStore> Iterator for Iter<'a, T> {
    type Item = Symbol;
    fn next(&mut self) -> Option<Self::Item> {
        let first = *self.0.next()?;
        let second = *self.0.next()?;
        Some((first, second).into())
    }
}

impl<T: BitStore> std::iter::FromIterator<i8> for SymbolVec<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = i8>,
    {
        iter.into_iter()
            .map(|s| Into::<Symbol>::into(s))
            .collect()
    }
}

impl<T: BitStore> std::iter::FromIterator<Symbol> for SymbolVec<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Symbol>,
    {
        iter.into_iter()
            .map(|s| Into::<(bool, bool)>::into(s))
            .collect()
    }
}

impl<T: BitStore> std::iter::FromIterator<(bool, bool)> for SymbolVec<T> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (bool, bool)>,
    {
        let mut inner = BitVec::new();
        let mut iter = iter.into_iter();
        loop {
            if let Some((first, second)) = iter.next() {
                inner.push(first);
                inner.push(second);
            } else {
                break;
            }
        }
        inner.shrink_to_fit();
        Self(inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbolvec_test() {
        let reference = vec![0i8, -1, 1, 0, -1, -1, 0, 0, 1];
        let symbol_vec: SymbolVec<u64> = reference.iter().cloned().collect();
        let result = symbol_vec.into_iter().map(|v| v as i8).collect::<Vec<_>>();
        assert_eq!(reference, result);

        let symbol_vec: SymbolVec<u8> = reference.iter().cloned().collect();
        let result = symbol_vec.into_iter().map(|v| v as i8).collect::<Vec<_>>();
        assert_eq!(reference, result);
    } 
}
