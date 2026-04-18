use governor::{
    clock::DefaultClock,
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::num::NonZeroU32;
use std::sync::Arc;

type Lim = RateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>;

#[derive(Clone)]
pub struct Throttle {
    inner: Option<Arc<Lim>>,
}

impl Throttle {
    #[must_use]
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn per_second(rps: u32) -> Self {
        let rps = NonZeroU32::new(rps.max(1)).unwrap();
        let lim = RateLimiter::direct(Quota::per_second(rps));
        Self {
            inner: Some(Arc::new(lim)),
        }
    }

    pub async fn acquire(&self) {
        if let Some(l) = &self.inner {
            l.until_ready().await;
        }
    }
}

impl std::fmt::Debug for Throttle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Throttle")
            .field("enabled", &self.inner.is_some())
            .finish()
    }
}
