//! IPC 类型导出测试：`cargo test export_bindings` 生成前端 TS 类型（PLAN §3.8）。
//! 生成的 bindings.ts 由 tauri-specta 维护，勿手改。

#[cfg(test)]
mod tests {
    use specta_typescript::Typescript;

    #[test]
    fn export_bindings() {
        let builder = crate::commands::specta_builder();
        builder
            .export(Typescript::default(), "../src/core/ipc/bindings.ts")
            .expect("Failed to export TypeScript bindings to src/core/ipc/bindings.ts");
    }
}
