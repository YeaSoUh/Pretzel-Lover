use std::fmt;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};
use std::str::FromStr;

use sea_query::{ArrayType, ColumnType, Value, ValueType, ValueTypeErr};

const SCALE: i64 = 100;

/// Fixed-point decimal with exactly 2 fractional digits, stored as i64 hundredths.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Cash(i64);

impl Cash {
    pub const ZERO: Self = Self(0);
    pub const MAX: Self = Self(i64::MAX);
    pub const MIN: Self = Self(i64::MIN);

    // ---- raw (hundredths): 12.34 <-> 1234 ----

    /// `from_raw(1234)` is `12.34`.
    pub const fn from_raw(raw: i64) -> Self {
        Self(raw)
    }

    /// The stored hundredths: `12.34` -> `1234`.
    pub const fn raw(self) -> i64 {
        self.0
    }

    // ---- whole (plain integer, no decimals): 12.34 <-> 12 ----

    /// `from_whole(12)` is `12.00`. `None` on overflow.
    pub const fn from_whole(whole: i64) -> Option<Self> {
        match whole.checked_mul(SCALE) {
            Some(c) => Some(Self(c)),
            None => None,
        }
    }

    /// Whole part, truncated toward zero: `12.34` -> `12`, `-12.34` -> `-12`, `-0.50` -> `0`.
    pub const fn whole(self) -> i64 {
        self.0 / SCALE
    }

    // ---- misc ----

    pub const fn is_negative(self) -> bool {
        self.0 < 0
    }

    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.0.checked_add(rhs.0) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.0.checked_sub(rhs.0) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    pub const fn checked_neg(self) -> Option<Self> {
        match self.0.checked_neg() {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// Multiply by a plain integer (exact, no rounding).
    pub const fn checked_mul_int(self, k: i64) -> Option<Self> {
        match self.0.checked_mul(k) {
            Some(v) => Some(Self(v)),
            None => None,
        }
    }

    /// `self * num / den`, rounded half away from zero.
    /// Covers percentages (`15, 100`), splitting (`1, n`), tax, etc.
    /// Uses i128 internally so the intermediate product can't overflow.
    pub fn checked_mul_ratio(self, num: i64, den: i64) -> Option<Self> {
        if den == 0 {
            return None;
        }
        let n = i128::from(self.0) * i128::from(num);
        let d = i128::from(den);
        let (q, r) = (n / d, n % d);
        let q = if r.abs() * 2 >= d.abs() {
            // Round away from zero: direction depends on the sign of the true quotient.
            if (n < 0) != (d < 0) { q - 1 } else { q + 1 }
        } else {
            q
        };
        <i64 as TryFrom<i128>>::try_from(q).ok().map(Self)
    }
}

// ---- operators (panic on overflow in ALL build profiles) ----

impl Add for Cash {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        self.checked_add(rhs).expect("Cash addition overflowed")
    }
}

impl Sub for Cash {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self.checked_sub(rhs).expect("Cash subtraction overflowed")
    }
}

impl Neg for Cash {
    type Output = Self;
    fn neg(self) -> Self {
        self.checked_neg().expect("Cash negation overflowed")
    }
}

impl Mul<i64> for Cash {
    type Output = Self;
    fn mul(self, k: i64) -> Self {
        self.checked_mul_int(k)
            .expect("Cash multiplication overflowed")
    }
}

impl AddAssign for Cash {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Cash {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl Sum for Cash {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |a, b| a + b)
    }
}

impl<'a> Sum<&'a Cash> for Cash {
    fn sum<I: Iterator<Item = &'a Cash>>(iter: I) -> Self {
        iter.copied().sum()
    }
}

// ---- formatting ----

impl fmt::Display for Cash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // unsigned_abs so i64::MIN doesn't blow up
        let abs = self.0.unsigned_abs();
        let scale = SCALE as u64;

        if f.alternate() {
            // {:#} -> whole part only, truncated: 12.99 -> "12"
            let whole = abs / scale;
            // `|| whole == 0` avoids printing "-0" for values like -0.50
            return f.pad_integral(self.0 >= 0 || whole == 0, "", &whole.to_string());
        }

        let digits = format!("{}.{:02}", abs / scale, abs % scale);
        // pad_integral handles sign, `{:+}`, width, and zero-padding for us
        f.pad_integral(self.0 >= 0, "", &digits)
    }
}

impl fmt::Debug for Cash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Cash({self})")
    }
}

// ---- parsing ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseCashError {
    Empty,
    Invalid,
    TooManyDecimals,
    Overflow,
}

impl fmt::Display for ParseCashError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "empty string",
            Self::Invalid => "invalid decimal",
            Self::TooManyDecimals => "more than 2 decimal places",
            Self::Overflow => "value out of range",
        })
    }
}

impl std::error::Error for ParseCashError {}

fn digit(b: u8) -> i64 {
    i64::from(b - b'0')
}

impl FromStr for Cash {
    type Err = ParseCashError;

    /// Accepts `12`, `12.3`, `12.34`, `.5`, `-0.05`, `+1.00`.
    /// Rejects whitespace, exponents, and >2 decimals (never silently rounds).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseCashError::Empty);
        }
        let (neg, body) = if let Some(r) = s.strip_prefix('-') {
            (true, r)
        } else if let Some(r) = s.strip_prefix('+') {
            (false, r)
        } else {
            (false, s)
        };

        let (whole, frac) = body.split_once('.').unwrap_or((body, ""));
        if whole.is_empty() && frac.is_empty() {
            return Err(ParseCashError::Invalid);
        }
        let all_digits = |p: &str| p.bytes().all(|b| b.is_ascii_digit());
        if !all_digits(whole) || !all_digits(frac) {
            return Err(ParseCashError::Invalid);
        }
        if frac.len() > 2 {
            return Err(ParseCashError::TooManyDecimals);
        }

        // Only digits remain, so a parse failure here can only mean overflow.
        let whole: i64 = if whole.is_empty() {
            0
        } else {
            whole.parse().map_err(|_| ParseCashError::Overflow)?
        };
        let frac_raw = match frac.as_bytes() {
            [] => 0,
            [a] => digit(*a) * 10,
            [a, b] => digit(*a) * 10 + digit(*b),
            _ => unreachable!(),
        };

        let whole_raw = whole.checked_mul(SCALE).ok_or(ParseCashError::Overflow)?;
        // Subtracting for negatives (instead of negating at the end) lets i64::MIN round-trip.
        let total = if neg {
            (-whole_raw).checked_sub(frac_raw)
        } else {
            whole_raw.checked_add(frac_raw)
        };
        total.map(Self).ok_or(ParseCashError::Overflow)
    }
}

impl From<Cash> for Value {
    fn from(c: Cash) -> Value {
        Value::BigInt(Some(c.raw()))
    }
}

impl ValueType for Cash {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        match v {
            Value::BigInt(Some(raw)) => Ok(Cash::from_raw(raw)),
            _ => Err(ValueTypeErr),
        }
    }

    fn type_name() -> String {
        stringify!(Cash).to_owned()
    }

    fn array_type() -> ArrayType {
        ArrayType::BigInt
    }

    fn column_type() -> ColumnType {
        ColumnType::BigInteger
    }
}
