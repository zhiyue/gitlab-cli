use rand::Rng;
use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub max_attempts_429: u32,
    pub base_ms: u64,
    pub factor: u32,
    pub jitter_pct: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self { max_attempts: 4, max_attempts_429: 5, base_ms: 500, factor: 2, jitter_pct: 20 }
    }
}

#[derive(Debug, Clone)]
pub struct RetryPlan {
    pub attempts: Vec<Duration>,
}

impl RetryPolicy {
    #[must_use]
    pub fn plan_for_network(&self) -> RetryPlan {
        self.plan(self.max_attempts)
    }

    #[must_use]
    pub fn plan_for_429(&self) -> RetryPlan {
        self.plan(self.max_attempts_429)
    }

    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    fn plan(&self, attempts: u32) -> RetryPlan {
        let mut rng = rand::thread_rng();
        let mut out = Vec::with_capacity(attempts as usize);
        for i in 0..attempts {
            let base = self.base_ms * u64::from(self.factor.pow(i));
            let jitter_range = base * u64::from(self.jitter_pct) / 100;
            let j: i64 = rng.gen_range(-(jitter_range as i64)..=(jitter_range as i64));
            let v = (base as i64 + j).max(0) as u64;
            out.push(Duration::from_millis(v));
        }
        RetryPlan { attempts: out }
    }

    #[must_use]
    pub fn next_delay_for_429(&self, retry_after: Option<&str>, attempt_idx: usize) -> Option<Duration> {
        if let Some(s) = retry_after {
            if let Ok(secs) = s.trim().parse::<u64>() {
                return Some(Duration::from_secs(secs));
            }
        }
        self.plan_for_429().attempts.get(attempt_idx).copied()
    }
}
