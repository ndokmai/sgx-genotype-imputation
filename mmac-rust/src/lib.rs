#![feature(const_fn)]

#[cfg(feature = "leak-resistant")]
mod bacc;
//#[cfg(feature = "leak-resistant")]
//mod const_time;
mod impute;
mod input;
mod ref_panel;
mod symbol;

#[cfg(feature = "leak-resistant")]
mod ln_fixed;
#[cfg(feature = "leak-resistant")]
mod ln_wrapped;
#[cfg(feature = "leak-resistant")]
mod wrapped;

pub use crate::impute::*;
pub use crate::input::*;
pub use crate::ref_panel::*;

#[cfg(not(feature = "leak-resistant"))]
mod inner {
    pub type Real = f64;
    pub type Input = i8;
}

#[cfg(feature = "leak-resistant")]
mod inner {
    use super::*;
    pub type Real = ln_fixed::LnFixed<typenum::U20>;
    //pub type Real = ln_wrapped::LnWrapped;
    pub type Input = timing_shield::TpI8;
}

use inner::*;
