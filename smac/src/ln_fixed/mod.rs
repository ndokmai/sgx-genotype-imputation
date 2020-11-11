#![allow(dead_code)]
mod fixed_inner_32;
mod fixed_inner_64;
mod implement;
use fixed_inner_32::*;
use fixed_inner_64::*;
pub use implement::TpLnFixed;
