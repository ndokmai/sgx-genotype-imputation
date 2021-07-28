#![feature(const_fn_transmute)]
#![feature(const_fn_floating_point_arithmetic)]
#![feature(const_mut_refs)]
#![feature(const_fn_trait_bound)]
#![allow(dead_code)]

mod fixed_inner_32;
mod fixed_inner_64;
mod implement;
use fixed_inner_32::*;
use fixed_inner_64::*;
pub use implement::TpLnFixed;
pub use timing_shield;
pub use num_traits;
