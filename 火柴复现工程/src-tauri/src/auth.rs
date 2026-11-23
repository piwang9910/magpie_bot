//! 身份接口（PLAN §3.6 / REQUIREMENTS P2-12）：v1 只有"本机本人"实现，
//! 不做登录 UI。将来权限/商业化挂接在此接口之后。

/// 身份标识。v1 固定为本机本人。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub id: String,
    pub display_name: String,
}

/// 身份提供者接口，预留登录/授权扩展点。
pub trait IdentityProvider: Send + Sync {
    fn current(&self) -> Identity;
    fn is_authenticated(&self) -> bool;
}

/// 本机身份：v1 唯一实现。
#[derive(Debug, Default)]
pub struct LocalIdentityProvider;

impl IdentityProvider for LocalIdentityProvider {
    fn current(&self) -> Identity {
        Identity {
            id: "local".into(),
            display_name: "本机本人".into(),
        }
    }

    fn is_authenticated(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_identity_is_authenticated() {
        let provider = LocalIdentityProvider;
        assert!(provider.is_authenticated());
        assert_eq!(provider.current().id, "local");
    }
}
