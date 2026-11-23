//! 本地搜索后端抽象（PLAN §3.1 / ADR-3）。
//! SearchBackend trait 定义统一查询接口；平台实现集中在各自文件。
//! M0 只定义 trait 与占位实现，真正搜索在 M2/M5。

use crate::models::{Query, SearchResult};

/// 本地搜索后端统一接口。换后端只改实现，不动调度器。
pub trait SearchBackend: Send + Sync {
    /// 后端名称。
    fn name(&self) -> &'static str;
    /// 后端是否可用（如 Everything 是否运行、mdfind 是否存在）。
    fn is_available(&self) -> bool;
    /// 查询；返回按相关度粗排的结果（评分层在 M2 实现）。
    fn search(&self, query: &Query) -> Result<Vec<SearchResult>, SearchError>;
}

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("搜索后端不可用: {0}")]
    Unavailable(String),
    #[error("查询失败: {0}")]
    QueryFailed(String),
}

/// 占位实现：永远返回空结果，表示"接口位已注册，实现待后续里程碑"。
#[derive(Debug, Default)]
pub struct StubBackend;

impl SearchBackend for StubBackend {
    fn name(&self) -> &'static str {
        "stub (M0 占位)"
    }

    fn is_available(&self) -> bool {
        false
    }

    fn search(&self, _query: &Query) -> Result<Vec<SearchResult>, SearchError> {
        Ok(Vec::new())
    }
}

/// 按平台返回默认后端（M0 全部返回占位实现）。
pub fn default_backend() -> StubBackend {
    StubBackend
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_backend_is_unavailable_and_empty() {
        let backend = default_backend();
        assert!(!backend.is_available());
        let results = backend
            .search(&Query {
                text: "anything".into(),
                limit: 20,
            })
            .unwrap();
        assert!(results.is_empty());
    }
}
