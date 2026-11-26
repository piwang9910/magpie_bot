//! 设备匹配码注册表（本地身份绑定，无账号体系）。
//!
//! 设计来源（公开资料，不含任何项目源码）：
//! - 微信输入法「跨设备复制粘贴 / 隔空传送」的公开机制：无需登录账号，一台设备生成
//!   6 位匹配码，另一台设备在 5 分钟内输入该码，两台设备即完成双向绑定。
//!
//! 工程化取舍：
//! - 匹配码使用系统随机源生成，拒绝采样避免取模偏差；
//! - 匹配码 5 分钟过期、成功即失效（单次使用），同一设备重新生成会作废旧码；
//! - 绑定关系是双向的，`is_bound(a, b)` 与 `is_bound(b, a)` 必须一致；
//! - 按请求设备限制失败次数，超限后进入短暂冷却，缓解局域网内的暴力猜测；
//! - 比较匹配码时使用常量时间比较，避免通过耗时差异逐位试探；
//! - 本模块只维护内存态控制面，不联网、不持久化设备信息；真实传输需要后续里程碑
//!   在 TLS / 设备证书层完成身份认证与限速。

use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::time::{Duration, Instant};

use thiserror::Error;

/// 匹配码长度。
pub const PAIRING_CODE_LEN: usize = 6;

/// 匹配码有效期。
pub const PAIRING_CODE_TTL: Duration = Duration::from_secs(5 * 60);

/// 单个请求设备在有效窗口内允许的失败次数。
pub const MAX_PAIRING_ATTEMPTS: u8 = 5;

/// 匹配码随机源。生产实现使用系统随机数；测试可注入固定序列。
pub trait CodeSource: Send {
    /// 生成一个候选匹配码；随机源不可用时返回 `None`。
    fn next_code(&mut self) -> Option<String>;
}

/// 基于操作系统随机源的匹配码生成器。
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemCodeSource;

impl CodeSource for SystemCodeSource {
    fn next_code(&mut self) -> Option<String> {
        let mut code = String::with_capacity(PAIRING_CODE_LEN);
        while code.len() < PAIRING_CODE_LEN {
            let mut byte = [0u8; 1];
            getrandom::fill(&mut byte).ok()?;
            // 250 是 10 的整数倍，只接受 0..=249 可保证 0..=9 等概率，避免取模偏差。
            if byte[0] < 250 {
                code.push(char::from(b'0' + byte[0] % 10));
            }
        }
        Some(code)
    }
}

/// 匹配码相关错误。
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PairingError {
    #[error("设备 ID 不能为空")]
    EmptyDeviceId,
    #[error("匹配码格式无效：应为 6 位数字")]
    MalformedCode,
    #[error("匹配码不存在或已被使用")]
    InvalidCode,
    #[error("匹配码已过期")]
    Expired,
    #[error("失败次数过多，请稍后再试")]
    TooManyAttempts,
    #[error("不能与本机自身配对")]
    SelfBinding,
    #[error("系统随机源不可用")]
    EntropyUnavailable,
}

/// 新生成的匹配码。Debug 输出会隐藏具体数字，避免日志泄露。
#[derive(Clone, PartialEq, Eq)]
pub struct IssuedCode {
    pub code: String,
    pub expires_in: Duration,
}

impl fmt::Debug for IssuedCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("IssuedCode")
            .field("code", &"******")
            .field("expires_in", &self.expires_in)
            .finish()
    }
}

/// 一次成功配对产生的双向绑定。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    /// 出示匹配码的设备。
    pub owner: String,
    /// 输入匹配码的设备。
    pub peer: String,
}

/// 内部票据。
struct Ticket {
    owner: String,
    expires_at: Instant,
}

/// 失败计数窗口。
struct FailureRecord {
    count: u8,
    window_start: Instant,
}

/// 设备匹配码注册表。
pub struct PairingRegistry<S: CodeSource = SystemCodeSource> {
    source: S,
    tickets: HashMap<String, Ticket>,
    bindings: HashMap<String, BTreeSet<String>>,
    failures: HashMap<String, FailureRecord>,
}

impl PairingRegistry<SystemCodeSource> {
    pub fn new() -> Self {
        Self::with_source(SystemCodeSource)
    }
}

impl Default for PairingRegistry<SystemCodeSource> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: CodeSource> PairingRegistry<S> {
    pub fn with_source(source: S) -> Self {
        Self {
            source,
            tickets: HashMap::new(),
            bindings: HashMap::new(),
            failures: HashMap::new(),
        }
    }

    /// 为 `owner` 生成一个新的匹配码；作废该设备之前未使用的匹配码。
    pub fn issue(&mut self, owner: &str, now: Instant) -> Result<IssuedCode, PairingError> {
        let owner = owner.trim();
        if owner.is_empty() {
            return Err(PairingError::EmptyDeviceId);
        }

        self.purge_expired(now);
        // 同一设备只保留最新匹配码，旧码立即失效。
        self.tickets.retain(|_, ticket| ticket.owner != owner);

        let mut generated = None;
        // 极小概率与现存匹配码碰撞，重试若干次仍失败则视为随机源异常。
        for _ in 0..16 {
            let candidate = self
                .source
                .next_code()
                .ok_or(PairingError::EntropyUnavailable)?;
            if is_well_formed(&candidate) && !self.tickets.contains_key(&candidate) {
                generated = Some(candidate);
                break;
            }
        }
        let code = generated.ok_or(PairingError::EntropyUnavailable)?;

        self.tickets.insert(
            code.clone(),
            Ticket {
                owner: owner.to_string(),
                expires_at: now + PAIRING_CODE_TTL,
            },
        );
        Ok(IssuedCode {
            code,
            expires_in: PAIRING_CODE_TTL,
        })
    }

    /// 请求设备输入匹配码完成绑定。
    pub fn redeem(
        &mut self,
        input: &str,
        requester: &str,
        now: Instant,
    ) -> Result<Binding, PairingError> {
        let requester = requester.trim();
        if requester.is_empty() {
            return Err(PairingError::EmptyDeviceId);
        }
        let input = input.trim();
        if !is_well_formed(input) {
            return Err(PairingError::MalformedCode);
        }
        if self.failure_count(requester, now) >= MAX_PAIRING_ATTEMPTS {
            return Err(PairingError::TooManyAttempts);
        }

        // 遍历所有活跃票据并做常量时间比较，避免通过响应耗时定位匹配项。
        let mut matched_code = None;
        for code in self.tickets.keys() {
            if constant_time_eq(code.as_bytes(), input.as_bytes()) {
                matched_code = Some(code.clone());
            }
        }

        let code = match matched_code {
            Some(code) => code,
            None => {
                self.record_failure(requester, now);
                return Err(PairingError::InvalidCode);
            }
        };

        let (owner, expired) = match self.tickets.get(&code) {
            Some(ticket) => (ticket.owner.clone(), now >= ticket.expires_at),
            None => {
                self.record_failure(requester, now);
                return Err(PairingError::InvalidCode);
            }
        };

        if expired {
            self.tickets.remove(&code);
            return Err(PairingError::Expired);
        }
        if owner == requester {
            self.record_failure(requester, now);
            return Err(PairingError::SelfBinding);
        }

        // 匹配码单次使用：成功即销毁，并清除请求设备的失败计数。
        self.tickets.remove(&code);
        self.failures.remove(requester);
        self.bindings
            .entry(owner.clone())
            .or_default()
            .insert(requester.to_string());
        self.bindings
            .entry(requester.to_string())
            .or_default()
            .insert(owner.clone());

        Ok(Binding {
            owner,
            peer: requester.to_string(),
        })
    }

    /// 判断两台设备是否已双向绑定。
    pub fn is_bound(&self, left: &str, right: &str) -> bool {
        if left == right {
            return false;
        }
        let forward = self
            .bindings
            .get(left)
            .map_or(false, |peers| peers.contains(right));
        let backward = self
            .bindings
            .get(right)
            .map_or(false, |peers| peers.contains(left));
        forward && backward
    }

    /// 返回某台设备已绑定的设备列表（按字典序，便于 UI 稳定展示）。
    pub fn bound_devices(&self, device: &str) -> Vec<String> {
        self.bindings
            .get(device)
            .map(|peers| peers.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// 解除双向绑定。
    pub fn unbind(&mut self, left: &str, right: &str) {
        if let Some(peers) = self.bindings.get_mut(left) {
            peers.remove(right);
            if peers.is_empty() {
                self.bindings.remove(left);
            }
        }
        if let Some(peers) = self.bindings.get_mut(right) {
            peers.remove(left);
            if peers.is_empty() {
                self.bindings.remove(right);
            }
        }
    }

    fn purge_expired(&mut self, now: Instant) {
        self.tickets.retain(|_, ticket| ticket.expires_at > now);
        self.failures
            .retain(|_, record| now.duration_since(record.window_start) < PAIRING_CODE_TTL);
    }

    fn failure_count(&self, requester: &str, now: Instant) -> u8 {
        match self.failures.get(requester) {
            Some(record) if now.duration_since(record.window_start) < PAIRING_CODE_TTL => {
                record.count
            }
            _ => 0,
        }
    }

    fn record_failure(&mut self, requester: &str, now: Instant) {
        let record = self
            .failures
            .entry(requester.to_string())
            .or_insert(FailureRecord {
                count: 0,
                window_start: now,
            });
        if now.duration_since(record.window_start) >= PAIRING_CODE_TTL {
            record.count = 0;
            record.window_start = now;
        }
        record.count = record.count.saturating_add(1);
    }
}

/// 匹配码必须是固定长度的纯数字。
fn is_well_formed(code: &str) -> bool {
    code.len() == PAIRING_CODE_LEN && code.bytes().all(|byte| byte.is_ascii_digit())
}

/// 常量时间字节比较。`black_box` 防止编译器把逐字节比较改写成提前返回。
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right) {
        diff = std::hint::black_box(diff | (a ^ b));
    }
    diff == 0
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::VecDeque;

    use super::*;

    /// 测试用固定序列随机源。
    pub(crate) struct FixedSource {
        codes: VecDeque<String>,
    }

    impl FixedSource {
        pub(crate) fn new(codes: &[&str]) -> Self {
            Self {
                codes: codes.iter().map(|code| code.to_string()).collect(),
            }
        }
    }

    impl CodeSource for FixedSource {
        fn next_code(&mut self) -> Option<String> {
            self.codes.pop_front()
        }
    }

    fn registry(codes: &[&str]) -> PairingRegistry<FixedSource> {
        PairingRegistry::with_source(FixedSource::new(codes))
    }

    #[test]
    fn redeem_binds_devices_in_both_directions() {
        let mut registry = registry(&["123456"]);
        let start = Instant::now();

        let issued = registry.issue("phone", start).expect("生成匹配码");
        assert_eq!(issued.code, "123456");
        assert_eq!(issued.expires_in, PAIRING_CODE_TTL);

        let binding = registry
            .redeem("123456", "pc", start + Duration::from_secs(60))
            .expect("配对成功");
        assert_eq!(binding.owner, "phone");
        assert_eq!(binding.peer, "pc");
        assert!(registry.is_bound("phone", "pc"));
        assert!(registry.is_bound("pc", "phone"));
        assert_eq!(registry.bound_devices("phone"), vec!["pc".to_string()]);
    }

    #[test]
    fn code_expires_after_five_minutes() {
        let mut registry = registry(&["123456"]);
        let start = Instant::now();
        registry.issue("phone", start).expect("生成匹配码");

        let result = registry.redeem(
            "123456",
            "pc",
            start + PAIRING_CODE_TTL + Duration::from_secs(1),
        );

        assert_eq!(result, Err(PairingError::Expired));
    }

    #[test]
    fn code_is_single_use() {
        let mut registry = registry(&["123456"]);
        let start = Instant::now();
        registry.issue("phone", start).expect("生成匹配码");
        registry
            .redeem("123456", "pc", start)
            .expect("首次配对成功");

        let second = registry.redeem("123456", "tablet", start);

        assert_eq!(second, Err(PairingError::InvalidCode));
    }

    #[test]
    fn too_many_failures_trigger_cooldown() {
        let mut registry = registry(&["123456"]);
        let start = Instant::now();
        registry.issue("phone", start).expect("生成匹配码");

        for _ in 0..MAX_PAIRING_ATTEMPTS {
            assert_eq!(
                registry.redeem("000000", "pc", start),
                Err(PairingError::InvalidCode)
            );
        }

        // 冷却期内即使输入正确匹配码也拒绝，避免继续消耗猜测次数。
        assert_eq!(
            registry.redeem("123456", "pc", start),
            Err(PairingError::TooManyAttempts)
        );
    }

    #[test]
    fn failure_window_resets_after_ttl() {
        let mut registry = registry(&["111111", "222222"]);
        let start = Instant::now();
        registry.issue("phone", start).expect("生成匹配码");

        for _ in 0..MAX_PAIRING_ATTEMPTS {
            assert_eq!(
                registry.redeem("000000", "pc", start),
                Err(PairingError::InvalidCode)
            );
        }

        let later = start + PAIRING_CODE_TTL + Duration::from_secs(1);
        let issued = registry.issue("phone", later).expect("重新生成匹配码");
        assert_eq!(issued.code, "222222");
        assert!(registry.redeem("222222", "pc", later).is_ok());
    }

    #[test]
    fn cannot_pair_with_self() {
        let mut registry = registry(&["123456"]);
        let start = Instant::now();
        registry.issue("phone", start).expect("生成匹配码");

        assert_eq!(
            registry.redeem("123456", "phone", start),
            Err(PairingError::SelfBinding)
        );
        assert!(!registry.is_bound("phone", "phone"));
    }

    #[test]
    fn new_code_invalidates_previous_code() {
        let mut registry = registry(&["111111", "222222"]);
        let start = Instant::now();
        registry.issue("phone", start).expect("生成第一个码");
        registry.issue("phone", start).expect("生成第二个码");

        assert_eq!(
            registry.redeem("111111", "pc", start),
            Err(PairingError::InvalidCode)
        );
        assert!(registry.redeem("222222", "pc", start).is_ok());
    }

    #[test]
    fn rejects_malformed_codes() {
        let mut registry = registry(&["123456"]);
        let start = Instant::now();
        registry.issue("phone", start).expect("生成匹配码");

        for malformed in ["12345", "1234567", "12a456", ""] {
            assert_eq!(
                registry.redeem(malformed, "pc", start),
                Err(PairingError::MalformedCode)
            );
        }
    }

    #[test]
    fn avoids_code_collision_with_active_ticket() {
        let mut registry = registry(&["111111", "111111", "222222"]);
        let start = Instant::now();
        registry.issue("phone", start).expect("生成第一个码");

        let second = registry.issue("tablet", start).expect("生成第二个码");

        assert_eq!(second.code, "222222");
        assert!(registry.redeem("111111", "pc", start).is_ok());
        assert!(registry.redeem("222222", "pc", start).is_ok());
        assert_eq!(registry.bound_devices("phone"), vec!["pc".to_string()]);
        assert_eq!(registry.bound_devices("tablet"), vec!["pc".to_string()]);
    }

    #[test]
    fn unbind_removes_both_directions() {
        let mut registry = registry(&["123456"]);
        let start = Instant::now();
        registry.issue("phone", start).expect("生成匹配码");
        registry.redeem("123456", "pc", start).expect("配对成功");

        registry.unbind("phone", "pc");

        assert!(!registry.is_bound("phone", "pc"));
        assert!(!registry.is_bound("pc", "phone"));
        assert!(registry.bound_devices("phone").is_empty());
    }

    #[test]
    fn entropy_failure_is_reported() {
        let mut registry = registry(&[]);
        assert_eq!(
            registry.issue("phone", Instant::now()),
            Err(PairingError::EntropyUnavailable)
        );
    }

    #[test]
    fn debug_redacts_code() {
        let issued = IssuedCode {
            code: "123456".into(),
            expires_in: PAIRING_CODE_TTL,
        };
        let rendered = format!("{issued:?}");
        assert!(rendered.contains("******"));
        assert!(!rendered.contains("123456"));
    }
}
