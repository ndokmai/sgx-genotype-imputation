mod impute;
mod input;
mod ref_panel;
mod symbol;

pub use crate::impute::*;
pub use crate::input::*;
pub use crate::ref_panel::*;

#[cfg(not(feature = "leak-resistant"))]
pub type Real = f64;
#[cfg(feature = "leak-resistant")]
pub type Real = ftfp::Fixed;
