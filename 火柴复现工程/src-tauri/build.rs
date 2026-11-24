fn main() {
    tauri_build::build();

    // Windows：tauri-runtime-wry / muda 静态导入 comctl32!TaskDialogIndirect，
    // 该符号只存在于由 Common-Controls 6.0 清单激活的 v6 版 comctl32。
    // 正式构建由 tauri-build 嵌入清单；cargo test 的测试目标没有清单，
    // 会在加载期直接报 STATUS_ENTRYPOINT_NOT_FOUND。这里把 comctl32 改为延迟加载，
    // 让未被调用的 UI 符号不影响测试进程启动。
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-arg=/DELAYLOAD:comctl32.dll");
        println!("cargo:rustc-link-arg=/DEFAULTLIB:delayimp.lib");
    }
}
