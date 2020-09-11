#[derive(PartialEq, Copy, Clone, Debug)]
pub enum Symbol {
    Ref,
    Alt,
    Missing,
}

impl std::convert::From<i8> for Symbol {
    fn from(symbol: i8) -> Self {
        match symbol {
            0 => Self::Ref,
            1 => Self::Alt,
            -1 => Self::Missing,
            _ => panic!("Invalid symbol"),
        }
    }
}

impl std::str::FromStr for Symbol {
    type Err = std::num::ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(s.parse::<i8>()?.into())
    }
}
