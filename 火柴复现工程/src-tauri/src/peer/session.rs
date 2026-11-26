//! 跨设备控制通道会话状态机（纯状态逻辑，不建立真实网络连接）。
//!
//! 设计来源（公开资料，不含任何项目源码）：
//! - Deskflow 公开协议参考：连接生命周期分为 Disconnected / Connecting /
//!   Handshake / Connected / Active；主控端发送“进入（CINN）”才开始转发输入，
//!   发送“离开（COUT）”后回到 Connected；保活间隔 3 秒，9 秒（3 倍间隔）收不到消息
//!   即断开。
//!
//! 这里只保留可验证的状态与超时判定，TCP / TLS / 输入事件注入由后续独立后台服务
//! 接入。把状态机先独立出来，可以避免网络层与平台钩子耦合后难以测试。

use std::time::{Duration, Instant};

use thiserror::Error;

/// 保活发送间隔。
pub const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(3);

/// 收不到任何消息后的断开时限。
pub const KEEPALIVE_TIMEOUT: Duration = Duration::from_secs(9);

/// 设备在会话中的角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerRole {
    /// 提供键鼠输入、统一维护布局的主控端。
    Primary,
    /// 接收并执行输入事件的受控端。
    Secondary,
}

/// 会话状态。与 Deskflow 公开的五态生命周期一一对应。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Disconnected,
    Connecting,
    Handshaking,
    Connected,
    Active,
}

/// 会话状态迁移错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SessionError {
    #[error("当前会话状态 {0:?} 不允许该操作")]
    InvalidTransition(SessionState),
    #[error("对端设备 ID 不能为空")]
    EmptyPeer,
}

/// 单条对等连接的会话状态机。
#[derive(Debug, Clone)]
pub struct Session {
    role: PeerRole,
    state: SessionState,
    peer: Option<String>,
    last_seen: Option<Instant>,
}

impl Session {
    pub fn new(role: PeerRole) -> Self {
        Self {
            role,
            state: SessionState::Disconnected,
            peer: None,
            last_seen: None,
        }
    }

    pub fn role(&self) -> PeerRole {
        self.role
    }

    pub fn state(&self) -> SessionState {
        self.state
    }

    pub fn peer(&self) -> Option<&str> {
        self.peer.as_deref()
    }

    /// 开始连接：仅允许从 Disconnected 进入。
    pub fn begin_connect(&mut self, peer: &str, now: Instant) -> Result<(), SessionError> {
        if self.state != SessionState::Disconnected {
            return Err(SessionError::InvalidTransition(self.state));
        }
        let peer = peer.trim();
        if peer.is_empty() {
            return Err(SessionError::EmptyPeer);
        }
        self.state = SessionState::Connecting;
        self.peer = Some(peer.to_string());
        self.last_seen = Some(now);
        Ok(())
    }

    /// 握手成功：Connecting → Connected。
    pub fn handshake_ok(&mut self, now: Instant) -> Result<(), SessionError> {
        if self.state != SessionState::Connecting {
            return Err(SessionError::InvalidTransition(self.state));
        }
        self.state = SessionState::Connected;
        self.last_seen = Some(now);
        Ok(())
    }

    /// 主控端允许输入进入：Connected → Active。
    pub fn enter(&mut self, now: Instant) -> Result<(), SessionError> {
        if self.state != SessionState::Connected {
            return Err(SessionError::InvalidTransition(self.state));
        }
        self.state = SessionState::Active;
        self.last_seen = Some(now);
        Ok(())
    }

    /// 输入离开：Active → Connected。
    pub fn leave(&mut self) -> Result<(), SessionError> {
        if self.state != SessionState::Active {
            return Err(SessionError::InvalidTransition(self.state));
        }
        self.state = SessionState::Connected;
        Ok(())
    }

    /// 关闭连接：任意状态均可回到 Disconnected。
    pub fn close(&mut self) {
        self.state = SessionState::Disconnected;
        self.peer = None;
        self.last_seen = None;
    }

    /// 收到任意消息或发送保活后刷新活跃时间。
    pub fn heartbeat(&mut self, now: Instant) {
        if matches!(self.state, SessionState::Connected | SessionState::Active) {
            self.last_seen = Some(now);
        }
    }

    /// 检查保活超时；超时会自动关闭会话并返回 `true`。
    pub fn tick(&mut self, now: Instant) -> bool {
        if !matches!(self.state, SessionState::Connected | SessionState::Active) {
            return false;
        }
        let timed_out = self.last_seen.map_or(true, |last_seen| {
            now.duration_since(last_seen) >= KEEPALIVE_TIMEOUT
        });
        if timed_out {
            self.close();
        }
        timed_out
    }

    /// 只有 Active 状态才允许把输入事件转发到对端。
    pub fn can_forward_input(&self) -> bool {
        self.state == SessionState::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connected_session() -> (Session, Instant) {
        let start = Instant::now();
        let mut session = Session::new(PeerRole::Primary);
        session.begin_connect("pc", start).expect("开始连接");
        session.handshake_ok(start).expect("握手成功");
        (session, start)
    }

    #[test]
    fn follows_the_documented_lifecycle() {
        let (mut session, start) = connected_session();
        assert_eq!(session.state(), SessionState::Connected);

        session.enter(start).expect("进入");
        assert!(session.can_forward_input());
        assert_eq!(session.state(), SessionState::Active);

        session.leave().expect("离开");
        assert!(!session.can_forward_input());
        assert_eq!(session.state(), SessionState::Connected);

        session.close();
        assert_eq!(session.state(), SessionState::Disconnected);
        assert_eq!(session.peer(), None);
    }

    #[test]
    fn rejects_invalid_transitions() {
        let start = Instant::now();
        let mut session = Session::new(PeerRole::Secondary);

        assert_eq!(
            session.handshake_ok(start),
            Err(SessionError::InvalidTransition(SessionState::Disconnected))
        );
        assert_eq!(
            session.enter(start),
            Err(SessionError::InvalidTransition(SessionState::Disconnected))
        );
        assert_eq!(
            session.leave(),
            Err(SessionError::InvalidTransition(SessionState::Disconnected))
        );

        session.begin_connect("pc", start).expect("开始连接");
        assert_eq!(
            session.enter(start),
            Err(SessionError::InvalidTransition(SessionState::Connecting))
        );
    }

    #[test]
    fn rejects_empty_peer() {
        let mut session = Session::new(PeerRole::Primary);
        assert_eq!(
            session.begin_connect("   ", Instant::now()),
            Err(SessionError::EmptyPeer)
        );
    }

    #[test]
    fn times_out_after_nine_seconds_without_activity() {
        let (mut session, start) = connected_session();
        session.enter(start).expect("进入");

        assert!(!session.tick(start + Duration::from_secs(8)));
        assert_eq!(session.state(), SessionState::Active);

        assert!(session.tick(start + KEEPALIVE_TIMEOUT));
        assert_eq!(session.state(), SessionState::Disconnected);
        assert_eq!(session.peer(), None);
    }

    #[test]
    fn heartbeat_resets_timeout() {
        let (mut session, start) = connected_session();
        session.enter(start).expect("进入");

        let heartbeat_at = start + Duration::from_secs(8);
        session.heartbeat(heartbeat_at);

        assert!(!session.tick(start + Duration::from_secs(16)));
        assert_eq!(session.state(), SessionState::Active);
        assert!(session.tick(heartbeat_at + KEEPALIVE_TIMEOUT));
    }

    #[test]
    fn timeout_is_three_keepalive_intervals() {
        assert_eq!(KEEPALIVE_TIMEOUT, KEEPALIVE_INTERVAL * 3);
    }
}
