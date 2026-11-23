//! 日志初始化。只记录耗时、状态码、错误类型，不记录用户输入（PLAN §3.9 / §9）。

use tracing_subscriber::EnvFilter;

pub fn init() {
    // RUST_LOG 未设置时默认 info 级别；日志不含输入内容。
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}
