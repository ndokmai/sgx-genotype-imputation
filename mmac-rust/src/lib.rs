#![feature(const_fn)]
#![feature(seek_convenience)]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]
#![allow(incomplete_features)]

pub mod block;
pub mod cache;
pub mod impute;
pub mod input;
pub mod ref_panel;
pub mod symbol;
pub mod tcp;

#[cfg(feature = "leak-resistant")]
mod bacc;
#[cfg(feature = "leak-resistant")]
mod ln_fixed;

pub use crate::cache::*;
pub use crate::impute::*;
pub use crate::input::*;
pub use crate::ref_panel::*;
pub use crate::symbol::*;
pub use crate::tcp::*;

#[cfg(not(feature = "leak-resistant"))]
mod inner {
    use super::*;
    pub type Real = f32;
    pub type Input = Symbol;
}

#[cfg(feature = "leak-resistant")]
mod inner {
    use super::*;
    pub type Real = ln_fixed::LnFixed<typenum::U20>;
    pub type Input = timing_shield::TpI8;
}

use inner::*;
