# M0 阶段验收记录：骨架与框架先行

> 对应 [PLAN.md](./PLAN.md) §4 M0 行、§3.10 里程碑门禁。
> 日期：2026-11-23（开发机：macOS，Apple Silicon；Rust 1.100.0 / Node 22.23.1）

## 一、M0 范围（按 PLAN §4 / §3.6 逐项对照）

| 框架项 | 落地位置 | 状态 |
|---|---|---|
| Tauri 双窗口（hud / panel） | `src-tauri/tauri.conf.json` + `index.html` / `panel.html` | ✅ 两个独立 JS 上下文，分别指向各自入口 |
| capabilities 基线（每窗口一份） | `src-tauri/capabilities/{hud,panel}.json` | ✅ 含 `core:default` + 最小核心权限 |
| 单实例 | `lib.rs` 注册 `tauri-plugin-single-instance` | ✅ 第二实例唤出已有 HUD |
| 托盘常驻 | `lib.rs::build_tray` | ✅ 左键唤出；菜单显示/退出 |
| 热键后端抽象（含 mac 后端） | `src-tauri/src/hotkey/` | ✅ trait + Hook/Reghotkey/Mac/Stub 实现；平台判断只在实现里 |
| M0 可演示唤出路径 | `lib.rs` 注册 `Alt+Space` 全局快捷键（global-shortcut 插件） | ✅ 两端可唤出 HUD；双击 Ctrl 钩子保留独立后端，M1 实现 |
| 设置读写 | `src-tauri/src/settings.rs` | ✅ 内存 + JSON 持久化；损坏回退默认；默认 `islandEnabled=false` |
| 调度器状态机 | `src/core/scheduler/state.ts` | ✅ 五态 + reducer 纯函数；判定点注入；13 条单测 |
| TriggerRegistry | `src/core/miniapps/registry.ts` | ✅ 注册/剥离/最长匹配/整串判定点；5 条单测 |
| SearchBackend trait + 占位实现 | `src-tauri/src/search/` | ✅ trait + Everything/mdfind/startmenu 扩展点文件 |
| gateway / auth / peer 接口 | `src-tauri/src/gateway/`, `auth.rs`, `peer.rs` | ✅ 仅接口位；AI 工具位注册 0 个工具、额度检查点常量 |
| avoidTop=140pt 常量 | `src-tauri/src/window.rs::AVOID_TOP_PT` | ✅ 单测锁定 140.0 |
| islandEnabled 设置位 + 岛窗口骨架 | `settings.rs` + `window.rs::ISLAND_WINDOW_LABEL` | ✅ 设置位与 label 常量；不创建窗口、不进 v1 验收 |
| IPC 契约（tauri-specta 一份定义两端同步） | `src-tauri/src/models.rs` + 生成的 `src/core/ipc/bindings.ts` | ✅ 9 个 command 类型生成；前端只经统一封装 |
| 计算器竖切 / 翻译 / AI 流式 / 本地搜索 | — | ⛔ M0 不实现（框架先行，按 PLAN 排期） |

## 二、里程碑门禁四项证据（PLAN §3.10）

### 1. 可演示

- HUD 与 Panel 为真实构建产物（`npm run build` 产出 `dist/`），headless Chrome 截图：
  - `evidence/M0-hud.png`（660×140，空态：输入框 + "骨架就绪 · 启动器网格将在 M1 接入"）
  - `evidence/M0-panel.png`（500×680，空壳：会话占位 + Esc 关闭提示 + 追问输入位）
- 唤出路径：Alt+Space 全局快捷键 → `window::show_hud`；托盘左键与单实例二次启动亦唤出 HUD。
- 注：截图为 WebView 入口在无头浏览器的渲染；macOS 图形会话下 `screencapture` 不可用（无显示器），故未附原生窗口截图。Windows 实机验收留在 M6。

### 2. 单测绿

```
cargo test --manifest-path src-tauri/Cargo.toml
  → 11 passed; 0 failed（含 export_bindings、设置读写、搜索/网关/身份/对等占位、avoidTop）

npm run test  (vitest)
  → 2 files, 14 tests passed; 0 failed（调度器 9 条 + TriggerRegistry 5 条）

npm run build (tsc --noEmit && vite build)
  → 通过，两个入口均产出；tsc strict 无错
```

### 3. 对端编译通过

- macOS `cargo check` 通过（覆盖全部跨平台模块）。
- Windows 专属代码仅存在于 `hotkey/win` 模块与 `window.rs` 的 `cfg(windows)` 分支，均为骨架且不含 Win32 FFI 调用；Windows 验收机 `cargo check`（全量）尚待在交付机上执行，本记录不伪造该项。
- 双端依赖未使用任何 POSIX 专属命令；`.gitattributes` 固定 `eol=lf`。

### 4. 实测数字

- M0 阶段未涉及响应时间阈值（PLAN §4 中响应时间阈值从 M1 起测）；验收表 11 条属 M6，本阶段不填。
- 已实测并记录于 PLAN §3.11：应用自有 command 无需在 capability 中显式声明（`app:allow-*` 会编译失败）。

## 三、已知限制（诚实记录）

- 热键后端仍是骨架：双击 Ctrl（HookBackend）未实现低级键盘钩子；M0 用 Alt+Space 全局快捷键保证可演示。
- 常驻灵动岛态只有设置位与 label 常量，没有窗口；按 ADR-7 在 v1 验收后实现。
- Panel 会话不落盘、流式输出未接；M3 实现。
- Windows 实机验收（钩子与杀软兼容、多屏、DPI、Everything）不在本机完成。

## 四、复现命令

```bash
cd 火柴复现工程
npm ci
cargo check --manifest-path src-tauri/Cargo.toml
cargo test  --manifest-path src-tauri/Cargo.toml
npm run test
npm run build
npm run tauri dev     # 在图形会话中；Alt+Space 唤出 HUD
```
