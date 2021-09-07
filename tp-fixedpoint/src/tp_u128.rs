use derive_more::*;
use timing_shield::{TpBool, TpCondSwap, TpEq, TpOrd, TpU64};

#[derive(From, BitXor, BitXorAssign, BitOr, BitOrAssign, BitAnd, Not, Clone, Copy)]
pub struct TpU128(u128);

impl TpU128 {
    #[inline(always)]
    pub fn protect(v: u128) -> Self {
        Self(v)
    }

    #[inline(always)]
    pub fn expose(self) -> u128 {
        self.0
    }
}

impl From<TpU64> for TpU128 {
    fn from(v: TpU64) -> Self {
        (v.expose() as u128).into()
    }
}

impl Into<TpU64> for TpU128 {
    fn into(self) -> TpU64 {
        TpU64::protect(self.0 as u64)
    }
}

impl TpOrd for TpU128 {
    fn tp_lt(&self, rhs: &Self) -> TpBool {
        let self_msb = TpU64::protect((self.0 >> 64) as u64);
        let self_lsb = TpU64::protect(self.0 as u64);
        let rhs_msb = TpU64::protect((rhs.0 >> 64) as u64);
        let rhs_lsb = TpU64::protect(rhs.0 as u64);
        self_msb.tp_lt(&rhs_msb) | (self_msb.tp_eq(&rhs_msb) & self_lsb.tp_lt(&rhs_lsb))
    }

    fn tp_gt(&self, rhs: &Self) -> TpBool {
        let self_msb = TpU64::protect((self.0 >> 64) as u64);
        let self_lsb = TpU64::protect(self.0 as u64);
        let rhs_msb = TpU64::protect((rhs.0 >> 64) as u64);
        let rhs_lsb = TpU64::protect(rhs.0 as u64);
        self_msb.tp_gt(&rhs_msb) | (self_msb.tp_eq(&rhs_msb) & self_lsb.tp_gt(&rhs_lsb))
    }

    fn tp_lt_eq(&self, rhs: &Self) -> TpBool {
        !self.tp_gt(rhs)
    }

    fn tp_gt_eq(&self, rhs: &Self) -> TpBool {
        !self.tp_lt(rhs)
    }
}

impl TpCondSwap for TpU128 {
    #[inline(always)]
    fn tp_cond_swap(condition: TpBool, a: &mut Self, b: &mut Self) {
        // Zero-extend condition to this type's width
        let cond_zx = Self(condition.expose() as u128);

        // Create mask of 11...11 for true or 00...00 for false
        let mask = !(cond_zx - 1);

        // swapper will be a XOR b for true or 00...00 for false
        let swapper = (*a ^ *b) & mask;

        *a ^= swapper;
        *b ^= swapper;
    }
}

impl std::ops::Sub<Self> for TpU128 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

impl std::ops::Sub<u128> for TpU128 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: u128) -> Self::Output {
        Self(self.0.wrapping_sub(rhs))
    }
}

impl std::ops::Shr<u32> for TpU128 {
    type Output = Self;
    #[inline(always)]
    fn shr(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_shr(rhs))
    }
}

impl std::ops::Shl<u32> for TpU128 {
    type Output = Self;
    #[inline(always)]
    fn shl(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_shl(rhs))
    }
}

impl std::ops::ShlAssign<u32> for TpU128 {
    #[inline(always)]
    fn shl_assign(&mut self, rhs: u32) {
        *self = *self << rhs;
    }
}
