//! 模型网关（PLAN §3.1 / §3.6）：DeepSeek 流式、Key 存取、取消令牌、额度检查点。
//! M0 只定义接口骨架与占位实现；真实 HTTP 流在 M3 实现。
//! 安全边界：Key 不进前端、不写日志。

use crate::models::ChatMessage;

/// 网关错误类型。
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("未配置 API Key")]
    MissingKey,
    #[error("请求被取消")]
    Cancelled,
    #[error("网关请求失败: {0}")]
    RequestFailed(String),
    #[error("额度检查点拦截: {0}（商业化挂接点）")]
    QuotaExceeded(String),
}

/// 取消令牌：用于中断流式输出。M0 只定义结构，M3 接入 tokio 取消。
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::SeqCst)
    }
}

/// 模型网关接口。翻译与 AI 直答共用。
pub trait ModelGateway: Send + Sync {
    fn name(&self) -> &'static str;
    /// 流式问答。M0 返回占位结果。
    fn stream_chat(
        &self,
        _messages: &[ChatMessage],
        _cancel: &CancelToken,
    ) -> Result<Vec<ChatMessage>, GatewayError>;
}

/// 占位网关：接口位已注册，实现待 M3。
#[derive(Debug, Default)]
pub struct StubGateway;

impl ModelGateway for StubGateway {
    fn name(&self) -> &'static str {
        "stub gateway (M0 占位)"
    }

    fn stream_chat(
        &self,
        _messages: &[ChatMessage],
        _cancel: &CancelToken,
    ) -> Result<Vec<ChatMessage>, GatewayError> {
        Err(GatewayError::MissingKey)
    }
}

/// AI 搜索子代理工具位（PLAN §3.6）：v1 注册零个 Search/Fetch 工具。
/// 硬上限"最多搜 3 次、抓 2 页"在此常量化，v2 实现时读取。
pub const MAX_SEARCH_CALLS: u32 = 3;
pub const MAX_FETCH_PAGES: u32 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_token_toggles() {
        let token = CancelToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn stub_gateway_has_name() {
        assert!(!StubGateway.name().is_empty());
    }
}
