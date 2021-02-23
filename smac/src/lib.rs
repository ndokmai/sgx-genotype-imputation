#![feature(const_fn)]
#![feature(const_mut_refs)]
#![feature(const_fn_transmute)]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]
#![feature(iter_partition_in_place)]
#![feature(const_fn_floating_point_arithmetic)]
#![allow(incomplete_features)]
#![cfg_attr(
    all(target_env = "sgx", target_vendor = "fortanix"),
    feature(sgx_platform)
)]

pub mod block;
pub mod client_input;
pub mod impute;
#[cfg(feature = "leak-resistant")]
pub mod ln_fixed;
pub mod ref_panel;
mod symbol;
pub mod symbol_vec;
pub mod tcp;

pub use crate::block::*;
pub use crate::client_input::*;
pub use crate::impute::*;
pub use crate::ref_panel::*;
pub use crate::symbol_vec::*;
pub use crate::tcp::*;

#[cfg(not(feature = "leak-resistant"))]
mod inner {
    use super::*;
    pub type Real = f32;
    pub type TargetSymbol = symbol::Symbol;
}

#[cfg(feature = "leak-resistant")]
mod inner {
    use super::*;
    pub type Real = ln_fixed::TpLnFixed<typenum::U20>;
    pub type TargetSymbol = timing_shield::TpI8;
}

pub use inner::*;
