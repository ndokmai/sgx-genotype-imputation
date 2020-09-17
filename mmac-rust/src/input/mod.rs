pub mod owned;
pub use owned::*;

use crate::Input;
use std::io::{Result, Write};

pub trait InputWriter {
    fn write(&mut self, writer: impl Write) -> Result<()>;
}

pub trait InputReader {
    type IndexIterator: Iterator<Item = bool>;
    type DataIterator: Iterator<Item = Input>;
    fn into_pair_iter(self) -> (Self::IndexIterator, Self::DataIterator);
}
