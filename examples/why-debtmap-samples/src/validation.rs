use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct Config {
    pub output_dir: Option<String>,
    pub max_workers: Option<usize>,
    pub timeout_secs: Option<u64>,
    pub retry_count: Option<u32>,
    pub log_level: Option<String>,
    pub cache_size: Option<usize>,
    pub batch_size: Option<usize>,
    pub max_connections: Option<usize>,
    pub buffer_size: Option<usize>,
    pub thread_pool_size: Option<usize>,
    pub queue_capacity: Option<usize>,
    pub heartbeat_interval: Option<u64>,
    pub session_timeout: Option<u64>,
    pub max_retries: Option<u32>,
    pub backoff_multiplier: Option<f64>,
    pub max_backoff: Option<u64>,
    pub request_timeout: Option<u64>,
    pub connect_timeout: Option<u64>,
    pub idle_timeout: Option<u64>,
    pub keepalive_interval: Option<u64>,
}

/// Function A: High Cyclomatic Complexity, Low Risk
/// This function has high cyclomatic complexity (20) but low cognitive complexity
/// because all branches follow the same repetitive pattern.
pub fn validate_config(config: &Config) -> Result<()> {
    if config.output_dir.is_none() {
        return Err(anyhow!("output_dir required"));
    }
    if config.max_workers.is_none() {
        return Err(anyhow!("max_workers required"));
    }
    if config.timeout_secs.is_none() {
        return Err(anyhow!("timeout_secs required"));
    }
    if config.retry_count.is_none() {
        return Err(anyhow!("retry_count required"));
    }
    if config.log_level.is_none() {
        return Err(anyhow!("log_level required"));
    }
    if config.cache_size.is_none() {
        return Err(anyhow!("cache_size required"));
    }
    if config.batch_size.is_none() {
        return Err(anyhow!("batch_size required"));
    }
    if config.max_connections.is_none() {
        return Err(anyhow!("max_connections required"));
    }
    if config.buffer_size.is_none() {
        return Err(anyhow!("buffer_size required"));
    }
    if config.thread_pool_size.is_none() {
        return Err(anyhow!("thread_pool_size required"));
    }
    if config.queue_capacity.is_none() {
        return Err(anyhow!("queue_capacity required"));
    }
    if config.heartbeat_interval.is_none() {
        return Err(anyhow!("heartbeat_interval required"));
    }
    if config.session_timeout.is_none() {
        return Err(anyhow!("session_timeout required"));
    }
    if config.max_retries.is_none() {
        return Err(anyhow!("max_retries required"));
    }
    if config.backoff_multiplier.is_none() {
        return Err(anyhow!("backoff_multiplier required"));
    }
    if config.max_backoff.is_none() {
        return Err(anyhow!("max_backoff required"));
    }
    if config.request_timeout.is_none() {
        return Err(anyhow!("request_timeout required"));
    }
    if config.connect_timeout.is_none() {
        return Err(anyhow!("connect_timeout required"));
    }
    if config.idle_timeout.is_none() {
        return Err(anyhow!("idle_timeout required"));
    }
    if config.keepalive_interval.is_none() {
        return Err(anyhow!("keepalive_interval required"));
    }
    Ok(())
}
