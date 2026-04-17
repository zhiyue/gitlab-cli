use tracing_subscriber::filter::EnvFilter;

#[must_use]
pub fn filter_for(v: Option<&str>) -> EnvFilter {
    match v {
        Some("1" | "info" | "INFO") => EnvFilter::new("info"),
        Some("debug" | "DEBUG") => EnvFilter::new("debug"),
        Some("trace" | "TRACE") => EnvFilter::new("trace"),
        _ => EnvFilter::new("warn"),
    }
}

pub fn init(v: Option<&str>) {
    let filter = filter_for(v);
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .without_time()
        .try_init();
}
