use crate::symbol::Symbol;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolVec {
    next_pos: usize,
    inner: Vec<u8>,
}

impl SymbolVec {
    pub fn new() -> Self {
        SymbolVec {
            next_pos: 0,
            inner: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        SymbolVec {
            next_pos: 0,
            inner: Vec::with_capacity(capacity >> 2),
        }
    }

    pub fn from_vec(inner: Vec<u8>) -> Self {
        let next_pos = inner.len() << 2;
        Self { inner, next_pos }
    }

    pub fn len(&self) -> usize {
        self.next_pos
    }

    pub fn push(&mut self, symbol: Symbol) {
        let symbol: u8 = symbol.into();
        let (byte, offset) = Self::pos_to_inner_pos(self.next_pos);
        if self.inner.len() <= byte {
            self.inner.push(0u8);
        }
        self.inner[byte] |= symbol << (offset << 1);
        self.next_pos += 1;
    }

    pub fn pop(&mut self) -> Option<Symbol> {
        if self.next_pos == 0 {
            return None;
        }
        self.next_pos -= 1;
        let (byte, offset) = Self::pos_to_inner_pos(self.next_pos);
        let target = &mut self.inner[byte];
        let bit_mask = 0b11 << (offset << 1);
        let symbol: Symbol = ((*target & bit_mask) >> (offset << 1)).into();
        *target &= !bit_mask;
        Some(symbol)
    }

    pub fn shrink_to_fit(&mut self) {
        let (byte, offset) = Self::pos_to_inner_pos(self.next_pos);
        self.inner.resize(byte + (offset != 0) as usize, 0);
        self.inner.shrink_to_fit();
    }

    pub fn reduce_size_to(&mut self, new_size: usize) {
        self.next_pos = new_size;
    }

    pub fn iter<'a>(&'a self) -> Iter<'a> {
        Iter {
            inner: self.inner.iter(),
            buffer: 0,
            buffer_len: 0,
            curr_len: self.len(),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.inner.as_slice()
    }

    /// Return (byte, offset)
    #[inline]
    fn pos_to_inner_pos(pos: usize) -> (usize, u8) {
        (pos >> 2, pos as u8 & 0b11)
    }
}

pub struct IntoIter {
    inner: std::vec::IntoIter<u8>,
    buffer: u8,
    buffer_len: usize,
    curr_len: usize,
}

impl Iterator for IntoIter {
    type Item = Symbol;
    fn next(&mut self) -> Option<Symbol> {
        if self.buffer_len > 0 {
            self.buffer_len -= 1;
            let out = self.buffer & 0b11;
            self.buffer >>= 2;
            Some(out.into())
        } else {
            if self.curr_len > 0 {
                self.buffer = self.inner.next().unwrap();
                if self.curr_len >= 4 {
                    self.buffer_len = 4;
                } else {
                    self.buffer_len = self.curr_len;
                }
                self.curr_len -= self.buffer_len;
                self.next()
            } else {
                None
            }
        }
    }
}

pub struct Iter<'a> {
    inner: std::slice::Iter<'a, u8>,
    buffer: u8,
    buffer_len: usize,
    curr_len: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = Symbol;
    fn next(&mut self) -> Option<Symbol> {
        if self.buffer_len > 0 {
            self.buffer_len -= 1;
            let out = self.buffer & 0b11;
            self.buffer >>= 2;
            Some(out.into())
        } else {
            if self.curr_len > 0 {
                self.buffer = *self.inner.next().unwrap();
                if self.curr_len >= 4 {
                    self.buffer_len = 4;
                } else {
                    self.buffer_len = self.curr_len;
                }
                self.curr_len -= self.buffer_len;
                self.next()
            } else {
                None
            }
        }
    }
}

impl IntoIterator for SymbolVec {
    type Item = Symbol;
    type IntoIter = IntoIter;
    fn into_iter(mut self) -> Self::IntoIter {
        self.shrink_to_fit();
        IntoIter {
            inner: self.inner.into_iter(),
            buffer: 0,
            buffer_len: 0,
            curr_len: self.next_pos,
        }
    }
}

impl std::iter::FromIterator<Symbol> for SymbolVec {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Symbol>,
    {
        let mut curr_len = 0;
        let mut inner = Vec::new();
        let mut byte = 0u8;
        let mut buffer_len = 0;
        for s in iter {
            curr_len += 1;
            let s: u8 = s.into();
            byte |= s << (buffer_len << 1);
            buffer_len += 1;
            if buffer_len == 4 {
                inner.push(byte);
                byte = 0u8;
                buffer_len = 0;
            }
        }
        if buffer_len != 0 {
            inner.push(byte);
        }
        Self {
            inner,
            next_pos: curr_len,
        }
    }
}

impl std::iter::FromIterator<i8> for SymbolVec {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = i8>,
    {
        iter.into_iter().map(|v| Into::<Symbol>::into(v)).collect()
    }
}

impl std::iter::FromIterator<u8> for SymbolVec {
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = u8>,
    {
        iter.into_iter().map(|v| Into::<Symbol>::into(v)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbolvec_push_pop() {
        let reference = vec![0i8, -1, 1, 0, -1, -1, 0, 0, 1];
        let mut symbol_vec = SymbolVec::new();
        for &i in &reference {
            symbol_vec.push(i.into());
        }
        symbol_vec.push(Symbol::Missing);
        symbol_vec.pop().unwrap();
        for &i in reference.iter().rev() {
            assert_eq!(symbol_vec.pop().unwrap(), i.into());
        }
        assert!(symbol_vec.pop().is_none());
        assert!(symbol_vec.inner.len() > 0);
        symbol_vec.shrink_to_fit();
        assert_eq!(symbol_vec.inner.len(), 0);
    }

    #[test]
    fn symbolvec_into_iter() {
        let reference = vec![0i8, -1, 1, 0, -1, -1, 0, 0, 1];
        let mut symbol_vec = SymbolVec::new();
        for &i in &reference {
            symbol_vec.push(i.into());
        }
        for (s, &r) in symbol_vec.into_iter().zip(&reference) {
            assert_eq!(s, r.into());
        }
    }

    #[test]
    fn symbolvec_from_iter() {
        let reference = vec![0i8, -1, 1, 0, -1, -1, 0, 0, 1];
        let symbol_vec: SymbolVec = reference.iter().cloned().collect();
        let result = symbol_vec.into_iter().map(|v| v as i8).collect::<Vec<_>>();
        assert_eq!(reference, result);
    }
}
