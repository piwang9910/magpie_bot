//! PeerService（PLAN §3.6 / REQUIREMENTS P2-13）：v1 空实现，仅接口位。
//! 返回"未启用"，将来作为独立后台服务接入，不侵入调度器。

/// 对等服务能力状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerStatus {
    Disabled,
    // 将来扩展：Discovering / Connected 等
}

/// 对等服务接口，预留局域网/文件快传扩展点。
pub trait PeerService: Send + Sync {
    fn status(&self) -> PeerStatus;
}

/// v1 空实现。
#[derive(Debug, Default)]
pub struct NoopPeerService;

impl PeerService for NoopPeerService {
    fn status(&self) -> PeerStatus {
        PeerStatus::Disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noop_peer_is_disabled() {
        let peer = NoopPeerService;
        assert_eq!(peer.status(), PeerStatus::Disabled);
    }
}
