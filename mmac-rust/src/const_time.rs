use crate::Real;
use timing_shield::{TpBool, TpU64};

#[inline]
fn f64_to_u64(x: f64) -> u64 {
    unsafe { *(&x as *const f64 as *const u64) }
}
#[inline]
fn u64_to_f64(x: u64) -> f64 {
    unsafe { *(&x as *const u64 as *const f64) }
}

#[allow(dead_code)]
#[inline]
pub fn select_2(cond: TpBool, a1: f64, a0: f64) -> Real {
    u64_to_f64(
        cond.select(
            TpU64::protect(f64_to_u64(a1)),
            TpU64::protect(f64_to_u64(a0)),
        )
        .expose(),
    )
    .into()
}

#[allow(dead_code)]
#[inline]
pub fn select_4(cond0: TpBool, cond1: TpBool, a11: f64, a10: f64, a01: f64, a00: f64) -> Real {
    let a0 = cond0
        .select(
            TpU64::protect(f64_to_u64(a10)),
            TpU64::protect(f64_to_u64(a00)),
        )
        .expose();
    let a1 = cond0
        .select(
            TpU64::protect(f64_to_u64(a11)),
            TpU64::protect(f64_to_u64(a01)),
        )
        .expose();
    u64_to_f64(
        cond1
            .select(TpU64::protect(a1), TpU64::protect(a0))
            .expose(),
    )
    .into()
}

#[inline]
pub fn select_4_no_ln(
    cond0: TpBool,
    cond1: TpBool,
    a11: f64,
    a10: f64,
    a01: f64,
    a00: f64,
) -> Real {
    let a0 = cond0
        .select(
            TpU64::protect(f64_to_u64(a10)),
            TpU64::protect(f64_to_u64(a00)),
        )
        .expose();
    let a1 = cond0
        .select(
            TpU64::protect(f64_to_u64(a11)),
            TpU64::protect(f64_to_u64(a01)),
        )
        .expose();
    Real::from_f64_no_ln(u64_to_f64(
        cond1
            .select(TpU64::protect(a1), TpU64::protect(a0))
            .expose(),
    ))
}
