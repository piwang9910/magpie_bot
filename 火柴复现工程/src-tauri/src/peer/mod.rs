//! PeerService（PLAN §3.6 / REQUIREMENTS P2-13）：跨设备协同控制面。
//!
//! 本模块把三个公开机制拆成可独立验证的纯逻辑子模块：
//! - [`layout`]：按真实物理毫米排列屏幕，12 mm 吸附、0.5 mm 接缝容差、仅绿边可穿越，
//!   旋转后沿相邻接缝重新锚定（设计来源：大象漫步公开说明）；
//! - [`pairing`]：6 位匹配码、5 分钟有效、单次使用、双向绑定的无账号身份配对
//!   （设计来源：微信输入法「隔空传送 / 跨设备」公开机制）；
//! - [`session`]：Disconnected → Connecting → Handshaking → Connected → Active
//!   五态生命周期、进入 / 离开与 3 秒保活 / 9 秒超时（设计来源：Deskflow 公开协议参考）。
//!
//! 边界与现状：
//! - 只实现控制面，不建立 TCP / TLS 连接、不注册平台输入钩子、不读写剪贴板；
//! - 这些能力依赖网络传输与平台权限，按 PLAN 的“框架先行”原则留给独立后台服务；
//! - 因此本模块不侵入调度器，也不改变现有 HUD / Panel 行为。

pub mod layout;
pub mod pairing;
pub mod session;

pub use layout::Layout;
pub use pairing::{Binding, IssuedCode, PairingError, PairingRegistry, SystemCodeSource};
pub use session::{PeerRole, Session, SessionState};

/// 对等服务能力状态，用于设置页或状态栏展示。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerStatus {
    /// 未启用（v1 占位实现）。
    Disabled,
    /// 已就绪，尚未建立会话。
    Idle,
    /// 正在配对或握手。
    Pairing,
    /// 控制通道已连通，但输入尚未进入本机。
    Connected,
    /// 输入正由对端转入本机。
    Active,
}

/// 对等服务接口，保留局域网 / 文件快传扩展点。
pub trait PeerService: Send + Sync {
    fn status(&self) -> PeerStatus;
}

/// 空实现：用于未启用对等能力的场景。
#[derive(Debug, Default)]
pub struct NoopPeerService;

impl PeerService for NoopPeerService {
    fn status(&self) -> PeerStatus {
        PeerStatus::Disabled
    }
}

/// 本地对等控制面：组合布局、匹配码与会话状态机。
///
/// 这是后续独立后台服务的核心装配点；网络层接入后只需在外部驱动
/// `session` 状态迁移，并把 `layout.try_cross` 的结果交给输入注入层。
pub struct PeerRuntime<S: pairing::CodeSource = pairing::SystemCodeSource> {
    local_device: String,
    layout: Layout,
    pairing: PairingRegistry<S>,
    session: Session,
}

impl<S: pairing::CodeSource> PeerRuntime<S> {
    pub fn new(
        local_device: impl Into<String>,
        role: PeerRole,
        pairing: PairingRegistry<S>,
    ) -> Self {
        Self {
            local_device: local_device.into(),
            layout: Layout::new(),
            pairing,
            session: Session::new(role),
        }
    }

    pub fn local_device(&self) -> &str {
        &self.local_device
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    pub fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    pub fn pairing(&self) -> &PairingRegistry<S> {
        &self.pairing
    }

    pub fn session(&self) -> &Session {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut Session {
        &mut self.session
    }

    /// 生成本机匹配码。
    pub fn issue_pairing_code(
        &mut self,
        now: std::time::Instant,
    ) -> Result<IssuedCode, PairingError> {
        self.pairing.issue(&self.local_device, now)
    }

    /// 输入对端匹配码完成双向绑定。
    pub fn redeem_pairing_code(
        &mut self,
        code: &str,
        peer_device: &str,
        now: std::time::Instant,
    ) -> Result<Binding, PairingError> {
        self.pairing.redeem(code, peer_device, now)
    }

    pub fn status(&self) -> PeerStatus {
        match self.session.state() {
            SessionState::Disconnected => PeerStatus::Idle,
            SessionState::Connecting | SessionState::Handshaking => PeerStatus::Pairing,
            SessionState::Connected => PeerStatus::Connected,
            SessionState::Active => PeerStatus::Active,
        }
    }
}

impl PeerRuntime<SystemCodeSource> {
    /// 以默认角色和系统随机源创建本机控制面。
    pub fn local(local_device: impl Into<String>) -> Self {
        Self::new(local_device, PeerRole::Primary, PairingRegistry::new())
    }
}

impl<S: pairing::CodeSource + Send + Sync> PeerService for PeerRuntime<S> {
    fn status(&self) -> PeerStatus {
        self.status()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::pairing::tests::FixedSource;
    use super::*;

    #[test]
    fn noop_peer_is_disabled() {
        assert_eq!(NoopPeerService.status(), PeerStatus::Disabled);
    }

    #[test]
    fn runtime_status_follows_session_state() {
        let mut runtime = PeerRuntime::new(
            "phone",
            PeerRole::Primary,
            PairingRegistry::with_source(FixedSource::new(&["123456"])),
        );
        assert_eq!(runtime.status(), PeerStatus::Idle);

        let start = Instant::now();
        runtime
            .session_mut()
            .begin_connect("pc", start)
            .expect("开始连接");
        assert_eq!(runtime.status(), PeerStatus::Pairing);

        runtime.session_mut().handshake_ok(start).expect("握手成功");
        assert_eq!(runtime.status(), PeerStatus::Connected);

        runtime.session_mut().enter(start).expect("进入");
        assert_eq!(runtime.status(), PeerStatus::Active);
        assert_eq!(runtime.session().state(), SessionState::Active);

        runtime.session_mut().close();
        assert_eq!(runtime.status(), PeerStatus::Idle);
    }

    #[test]
    fn runtime_delegates_pairing_to_local_device() {
        let mut runtime = PeerRuntime::new(
            "phone",
            PeerRole::Primary,
            PairingRegistry::with_source(FixedSource::new(&["654321"])),
        );
        let start = Instant::now();

        let issued = runtime.issue_pairing_code(start).expect("生成匹配码");
        assert_eq!(issued.code, "654321");

        let binding = runtime
            .redeem_pairing_code("654321", "pc", start + Duration::from_secs(10))
            .expect("配对成功");
        assert_eq!(binding.owner, "phone");
        assert!(runtime.pairing().is_bound("phone", "pc"));
    }
}
