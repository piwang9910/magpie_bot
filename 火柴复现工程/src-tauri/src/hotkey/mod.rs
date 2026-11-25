//! 热键后端抽象（PLAN §3.1 / ADR-1）。
//! 平台判断只出现在 trait 实现里；Windows 专属代码集中在 hook_win.rs / reghotkey_win.rs。

use std::sync::Arc;

use tracing::info;

use crate::models::HotkeyBackendKind;

/// 热键事件回调：当热键被触发时调用。
pub type HotkeyCallback = Arc<dyn Fn() + Send + Sync>;

/// 热键后端统一接口。两个窗口共用这一层抽象。
pub trait HotkeyBackend: Send + Sync {
    /// 后端名称（用于设置页展示当前生效后端）。
    fn name(&self) -> &'static str;
    /// 启动监听；注册热键/钩子。
    fn start(&self, callback: HotkeyCallback) -> Result<(), HotkeyError>;
    /// 停止监听并释放资源。
    fn stop(&self) -> Result<(), HotkeyError>;
}

#[derive(Debug, thiserror::Error)]
pub enum HotkeyError {
    #[error("热键已被占用或注册失败: {0}")]
    RegisterFailed(String),
    #[error("当前平台不支持此热键后端: {0}")]
    Unsupported(String),
}

/// 由 AppState 持有的热键后端句柄，保证后端实例在整个应用生命周期内存活，
/// 退出时可调用 stop() 释放钩子/热键资源（框架层职责）。
pub struct HotkeyHandle {
    pub backend: Arc<dyn HotkeyBackend>,
}

/// 根据设置中的后端类型创建对应实现。
pub fn create_backend(kind: HotkeyBackendKind) -> Arc<dyn HotkeyBackend> {
    match kind {
        #[cfg(target_os = "windows")]
        HotkeyBackendKind::Hook => Arc::new(win::HookBackend::new()),
        #[cfg(target_os = "windows")]
        HotkeyBackendKind::Reghotkey => Arc::new(win::RegHotkeyBackend::new()),
        // 设置里存的是 macOS 平台后端时（跨机器同步设置文件），Windows 回退到 reghotkey。
        #[cfg(target_os = "windows")]
        HotkeyBackendKind::Mac => Arc::new(win::RegHotkeyBackend::new()),
        #[cfg(target_os = "macos")]
        HotkeyBackendKind::Hook => Arc::new(mac::MacBackend::new("hook-fallback")),
        #[cfg(target_os = "macos")]
        HotkeyBackendKind::Reghotkey => Arc::new(mac::MacBackend::new("reghotkey-fallback")),
        #[cfg(target_os = "macos")]
        HotkeyBackendKind::Mac => Arc::new(mac::MacBackend::new("mac")),
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        HotkeyBackendKind::Hook
        | HotkeyBackendKind::Reghotkey
        | HotkeyBackendKind::Mac => Arc::new(stub::StubBackend),
    }
}

/// macOS 平台后端：M0 只搭骨架，真键位监听在后续里程碑接入。
/// 目前 start 只记录日志并返回成功，回调可通过 `trigger_for_dev()` 手动触发以冒烟。
#[cfg(target_os = "macos")]
pub mod mac {
    use super::*;

    pub struct MacBackend {
        label: &'static str,
    }

    impl MacBackend {
        pub fn new(label: &'static str) -> Self {
            Self { label }
        }
    }

    impl HotkeyBackend for MacBackend {
        fn name(&self) -> &'static str {
            match self.label {
                "mac" => "mac (开发后端)",
                "hook-fallback" => "mac (hook 回退)",
                _ => "mac (reghotkey 回退)",
            }
        }

        fn start(&self, _callback: HotkeyCallback) -> Result<(), HotkeyError> {
            info!(backend = self.label, "macOS 热键后端已启动（M0 骨架，尚未接入真实按键监听）");
            Ok(())
        }

        fn stop(&self) -> Result<(), HotkeyError> {
            info!(backend = self.label, "macOS 热键后端已停止");
            Ok(())
        }
    }
}

/// Windows 专属后端：M0 只留结构，真正钩子代码在后续里程碑实现。
/// 文件集中在此处，由 Windows 验收机兜底编译（PLAN §3.5 平台隔离纪律）。
#[cfg(target_os = "windows")]
pub mod win {
    use super::*;

    /// 低级键盘钩子：双击 Ctrl（ADR-1）。
    pub struct HookBackend;

    impl HookBackend {
        pub fn new() -> Self {
            Self
        }
    }

    impl HotkeyBackend for HookBackend {
        fn name(&self) -> &'static str {
            "hook (双击 Ctrl)"
        }

        fn start(&self, _callback: HotkeyCallback) -> Result<(), HotkeyError> {
            // TODO(M1+): WH_KEYBOARD_LL 钩子注册；两次 Ctrl 间出现其他键即作废。
            info!("Windows HookBackend 已启动（M0 骨架）");
            Ok(())
        }

        fn stop(&self) -> Result<(), HotkeyError> {
            Ok(())
        }
    }

    /// RegisterHotKey：Alt+Space 兜底（ADR-1）。
    pub struct RegHotkeyBackend;

    impl RegHotkeyBackend {
        pub fn new() -> Self {
            Self
        }
    }

    impl HotkeyBackend for RegHotkeyBackend {
        fn name(&self) -> &'static str {
            "reghotkey (Alt+Space)"
        }

        fn start(&self, _callback: HotkeyCallback) -> Result<(), HotkeyError> {
            // TODO(M1+): RegisterHotKey 注册 Alt+Space。
            info!("Windows RegHotkeyBackend 已启动（M0 骨架）");
            Ok(())
        }

        fn stop(&self) -> Result<(), HotkeyError> {
            Ok(())
        }
    }
}

/// 其他平台的占位后端。
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub mod stub {
    use super::*;

    pub struct StubBackend;

    impl HotkeyBackend for StubBackend {
        fn name(&self) -> &'static str {
            "stub"
        }
        fn start(&self, _callback: HotkeyCallback) -> Result<(), HotkeyError> {
            Ok(())
        }
        fn stop(&self) -> Result<(), HotkeyError> {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_backend_returns_named_backend() {
        let backend = create_backend(crate::models::default_hotkey_backend());
        assert!(!backend.name().is_empty());
    }
}
