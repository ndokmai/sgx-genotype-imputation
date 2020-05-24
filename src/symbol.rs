#[derive(Copy, Clone, Debug)]
pub enum Symbol {
    A = 0,
    C = 1,
    G = 2,
    T = 3,
    Missing = 4,
}

impl Symbol {
    pub fn parse(c: &char) -> Result<Self, &'static str> {
        Ok(
            match c {
                'A' => Self::A,
                'C' => Self::C,
                'G' => Self::G,
                'T' => Self::T,
                '.' => Self::Missing,
                _ => return Err("Invalid symbol character")
            }
          )
    }

    pub fn from_pos(i: usize) -> Result<Self, &'static str> {
        Ok(
            match i {
                0 => Self::A,
                1 => Self::C,
                2 => Self::G,
                3 => Self::T,
                4 => Self::Missing,
                _ => return Err("Invalid position")
            }
          )
    }

    pub fn pos(&self) -> usize {
        *self as usize
    }
}

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> { 
        match self {
            Self::Missing => write!(f, "."),
            _ => write!(f, "{:?}", self)
        }
    }

}
