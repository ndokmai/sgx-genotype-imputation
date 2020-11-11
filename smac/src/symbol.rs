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

impl From<u8> for Symbol {
    fn from(code: u8) -> Self {
        match code & 0b11 {
            0 => Self::Missing,
            1 => Self::Ref,
            2 => Self::Alt,
            _ => panic!("Invalid symbol"),
        }
    }
}

impl Into<u8> for Symbol {
    fn into(self) -> u8 {
        (self as i8 + 1) as u8
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
