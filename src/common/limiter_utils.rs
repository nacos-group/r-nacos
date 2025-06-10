use std::convert::TryFrom;
use std::fmt::Formatter;

use ratelimiter_rs::RateLimiter;

pub struct LimiterData {
    pub rate_to_ms_conversion: i32,
    pub consumed_tokens: i32,
    pub last_refill_time: i64,
}

impl LimiterData {
    pub fn new(rate_to_ms_conversion: i32, consumed_tokens: i32, last_refill_time: i64) -> Self {
        Self {
            rate_to_ms_conversion,
            consumed_tokens,
            last_refill_time,
        }
    }
    pub fn to_rate_limiter(self) -> RateLimiter {
        RateLimiter::load(
            self.rate_to_ms_conversion,
            self.consumed_tokens,
            self.last_refill_time,
        )
    }
}

impl TryFrom<&str> for LimiterData {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut iter = value.split(',');
        let a: i32 = if let Some(e) = iter.next() {
            e.parse()?
        } else {
            return Err(anyhow::anyhow!("limiter is invalid"));
        };
        let b: i32 = if let Some(e) = iter.next() {
            e.parse()?
        } else {
            return Err(anyhow::anyhow!("limiter is invalid"));
        };
        let c: i64 = if let Some(e) = iter.next() {
            e.parse()?
        } else {
            return Err(anyhow::anyhow!("limiter is invalid"));
        };
        Ok(Self::new(a, b, c))
    }
}

impl std::fmt::Display for LimiterData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},{}",
            self.rate_to_ms_conversion, self.consumed_tokens, self.last_refill_time
        )
    }
}

impl From<RateLimiter> for LimiterData {
    fn from(value: RateLimiter) -> Self {
        let (a, b, c) = value.get_to_save();
        Self::new(a, b, c)
    }
}
