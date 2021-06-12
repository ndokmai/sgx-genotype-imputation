#![cfg_attr(
    all(target_env = "sgx", target_vendor = "fortanix"),
    feature(sgx_platform)
)]

pub mod block;
pub mod client_input;
pub mod impute;
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
    pub type Real = tp_fixedpoint::TpLnFixed<tp_fixedpoint::typenum::U20>;
    pub type TargetSymbol = tp_fixedpoint::timing_shield::TpI8;
}

pub use inner::*;
