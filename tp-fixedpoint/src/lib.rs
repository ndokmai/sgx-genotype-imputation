#![feature(const_fn_floating_point_arithmetic)]
#![feature(const_mut_refs)]
#![feature(const_fn_trait_bound)]
#![allow(dead_code)]

pub mod fixed_inner_32;
pub mod fixed_inner_64;
mod implement;
use fixed_inner_32::*;
use fixed_inner_64::*;
pub use implement::TpLnFixed;
pub use num_traits;
pub use timing_shield;
