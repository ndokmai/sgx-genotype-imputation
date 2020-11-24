#![feature(const_fn)]
#![feature(const_mut_refs)]
#![feature(const_fn_transmute)]
#![feature(seek_convenience)]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]
#![feature(iter_partition_in_place)]
#![feature(const_fn_floating_point_arithmetic)]
#![allow(incomplete_features)]
#![cfg_attr(
    all(target_env = "sgx", target_vendor = "fortanix"),
    feature(sgx_platform)
)]

mod block;
pub mod cache;
pub mod impute;
pub mod input;
#[cfg(feature = "leak-resistant")]
mod ln_fixed;
pub mod output;
pub mod ref_panel;
mod symbol;
mod symbol_vec;
pub mod tcp;

pub use crate::cache::*;
pub use crate::impute::*;
pub use crate::input::*;
pub use crate::output::*;
pub use crate::ref_panel::*;
pub use crate::tcp::*;

#[cfg(not(feature = "leak-resistant"))]
mod inner {
    use super::*;
    pub type Real = f32;
    pub type Input = symbol::Symbol;
}

#[cfg(feature = "leak-resistant")]
mod inner {
    use super::*;
    pub type Real = ln_fixed::TpLnFixed<typenum::U20>;
    pub type Input = timing_shield::TpI8;
}

pub use inner::*;
