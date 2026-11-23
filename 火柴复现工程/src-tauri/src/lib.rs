//! 喜鹊助手 Tauri 主库（M0 骨架）。
//! 平台差异隔离在 trait 实现里；本文件只做装配。
//!
//! M0 是「框架先行」里程碑：auth / peer / gateway / search 等接口位已注册但
//! 尚未被业务调用，dead_code 警告是预期的，待 M1–M4 逐个实现后自然消除。
#![allow(dead_code)]

use std::sync::Arc;

use tauri::Manager;
use tracing::info;

mod auth;
mod commands;
mod gateway;
mod hotkey;
mod ipc_export;
mod logging;
mod models;
mod peer;
mod search;
mod settings;
mod window;

/// Tauri 应用入口：由 main.rs 调用。
pub fn run() {
    logging::init();

    let builder = commands::specta_builder();

    // M0 实机冒烟：在 debug 构建下把 IPC 类型导出到前端生成物。
    #[cfg(debug_assertions)]
    builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/core/ipc/bindings.ts",
        )
        .expect("Failed to export TypeScript bindings");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // 第二个实例启动时，直接唤出已有实例的 HUD。
            window::show_hud(app);
        }))
        // M0 可演示路径：注册 Alt+Space 全局快捷键，两端均可唤出 HUD。
        // 这与设置里的 reghotkey 后端一致；双击 Ctrl 钩子仍是独立后端，待 M1 实现。
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcut("Alt+Space")
                .expect("register Alt+Space shortcut")
                .with_handler(|app, _shortcut, event| {
                    use tauri_plugin_global_shortcut::ShortcutState;
                    if event.state == ShortcutState::Pressed {
                        info!("全局快捷键 Alt+Space → 唤出 HUD");
                        window::show_hud(app);
                    }
                })
                .build(),
        )
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            let handle = app.handle().clone();

            // 设置仓储注入全局 State（跨平台路径，无硬编码绝对路径）。
            let store = settings::SettingsStore::new(settings::default_config_dir());
            let backend_kind = store.load().hotkey_backend;
            app.manage(store);

            // 常驻灵动岛窗口骨架：仅注册结构，不显示、不实现功能（ADR-7）。
            // 在 M0 不创建真实窗口，只保留 label 常量与设置位。

            // 热键后端抽象：按设置选择实现；M0 各实现为骨架。
            let hotkey = hotkey::create_backend(backend_kind);
            // 把后端句柄交给 AppState 持有，避免 setup 返回后被 drop；
            // 退出钩子在 M1 接入真实钩子时通过此句柄调用 stop()。
            app.manage(hotkey::HotkeyHandle {
                backend: Arc::clone(&hotkey),
            });
            let callback: hotkey::HotkeyCallback = Arc::new(move || {
                info!("热键触发 → 唤出 HUD");
                window::show_hud(&handle);
            });
            hotkey.start(callback).map_err(|e| e.to_string())?;

            // 托盘常驻：点击左键唤出 HUD，双击/右键退出。
            build_tray(app)?;

            info!("喜鹊助手 M0 骨架启动完成");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 构建系统托盘。图标为 M0 占位（一个 1x1 透明 PNG），功能只要求"托盘常驻"。
fn build_tray(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let show = MenuItem::with_id(app, "show", "显示/隐藏", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let _tray = TrayIconBuilder::with_id("main")
        .icon(tauri::image::Image::new_owned(vec![0, 0, 0, 0], 1, 1))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => window::show_hud(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::show_hud(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}
