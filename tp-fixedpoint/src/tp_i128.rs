use derive_more::From;
use timing_shield::TpI64;

#[derive(From, Clone, Copy)]
pub struct TpI128(i128);

impl TpI128 {
    pub fn protect(v: i128)  -> Self {
        Self(v)
    }
}

impl From<TpI64> for TpI128 {
    fn from(v: TpI64) -> Self {
        (v.expose() as i128).into()
    }
}

impl Into<TpI64> for TpI128 {
    fn into(self) -> TpI64 {
        TpI64::protect(self.0 as i64)
    }
}

impl std::ops::Add<Self> for TpI128 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_add(rhs.0))
    }
}

impl std::ops::Sub<Self> for TpI128 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_sub(rhs.0))
    }
}

impl std::ops::Mul<Self> for TpI128 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0.wrapping_mul(rhs.0))
    }
}

impl std::ops::Shr<u32> for TpI128 {
    type Output = Self;
    #[inline]
    fn shr(self, rhs: u32) -> Self::Output {
        Self(self.0.wrapping_shr(rhs))
    }
}

impl num_traits::One for TpI128 {
    fn one() -> Self {
        Self(1)
    }
}
