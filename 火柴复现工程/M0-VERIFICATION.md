# M0 阶段验收记录：骨架与框架先行

> 对应 [PLAN.md](./PLAN.md) §4 M0 行、§3.10 里程碑门禁。
> 日期：2026-11-23（**历史记录 · macOS 时期**：开发机 macOS，Apple Silicon；Rust 1.100.0 / Node 22.23.1）
> 开发环境已于 2026-11-25 更正为 Windows 主机开发+验收，Windows 复验结果见 §五。

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
- 注：截图为 WebView 入口在无头浏览器的渲染；macOS 图形会话下 `screencapture` 不可用（无显示器），故该轮未附原生窗口截图。原生窗口实机验收改在 Windows 主机完成（见 §五）。

### 2. 单测绿

```
cargo test --manifest-path src-tauri/Cargo.toml
  → 11 passed; 0 failed（含 export_bindings、设置读写、搜索/网关/身份/对等占位、avoidTop）

npm run test  (vitest)
  → 2 files, 14 tests passed; 0 failed（调度器 9 条 + TriggerRegistry 5 条）

npm run build (tsc --noEmit && vite build)
  → 通过，两个入口均产出；tsc strict 无错
```

### 3. 全量编译通过（原「对端编译通过」）

- macOS `cargo check` 通过（覆盖全部跨平台模块）。
- Windows 专属代码仅存在于 `hotkey` 的 `cfg(windows)` 分支与 `window.rs` 的 `cfg(windows)` 分支，均为骨架且不含 Win32 FFI 调用。
- 开发环境更正后，Windows 主机已执行**全量** `cargo check`（含 `cfg(windows)` 分支），结果见 §五；macOS 降级为可迁移性回归。
- 双端依赖未使用任何 POSIX 专属命令；`.gitattributes` 固定 `eol=lf`。

### 4. 实测数字

- M0 阶段未涉及响应时间阈值（PLAN §4 中响应时间阈值从 M1 起测）；验收表 11 条属 M6，本阶段不填。
- 已实测并记录于 PLAN §3.11：应用自有 command 无需在 capability 中显式声明（`app:allow-*` 会编译失败）。

## 三、已知限制（诚实记录）

- 热键后端仍是骨架：双击 Ctrl（HookBackend）未实现低级键盘钩子；M0 用 Alt+Space 全局快捷键保证可演示。
- 常驻灵动岛态只有设置位与 label 常量，没有窗口；按 ADR-7 在 v1 验收后实现。
- Panel 会话不落盘、流式输出未接；M3 实现。
- Windows 实机验收（钩子与杀软兼容、多屏、DPI、Everything）已在 Windows 主机完成基础冒烟，深度验收留 M6。

## 四、复现命令

```powershell
cd 火柴复现工程
npm ci
cargo check --manifest-path src-tauri/Cargo.toml
cargo test  --manifest-path src-tauri/Cargo.toml
npm run test
npm run build
npm run tauri dev     # 首选 Alt+Space；被系统占用时按候选链回退（Ctrl+Alt+Space → Alt+Q），日志写明生效键
```

## 五、Windows 主机复验（2026-11-25）

开发环境更正后，在 Windows 主机（Node 24.22.1 / npm 11.19.1 / Rust 1.100.0 MSVC / VS 2022 Build Tools / WebView2 158）重跑全链路。

| 步骤 | 命令 | 结果 |
|---|---|---|
| 依赖安装 | `npm ci` | 通过，48 个包，0 漏洞；`@rolldown/binding-win32-x64-msvc`、`@tauri-apps/cli-win32-x64-msvc` 就位 |
| 前端单测 | `npm run test` | 通过，2 文件 / 14 用例 |
| 前端构建 | `npm run build` | 通过，`dist/index.html`、`dist/panel.html` 均产出 |
| 全量编译 | `cargo check --manifest-path src-tauri/Cargo.toml` | 通过，0 error（含 `cfg(target_os="windows")` 分支，这是 macOS 时期无法验证的部分） |
| Rust 单测 | `cargo test --manifest-path src-tauri/Cargo.toml` | 通过，11 passed；`export_bindings` 重新生成 `src/core/ipc/bindings.ts` |
| 开发模式 | `npm run tauri dev` | 启动成功，无 panic，HUD/Panel 入口正常加载 |
| 实机冒烟 | `magpie-bot.exe` | 见下 |

实机冒烟结论：

1. **Alt+Space 与 Windows 冲突已确认**：注册返回 `HotKey already registered: HotKey { mods: Modifiers(ALT), key: Space, id: 65598 }`（系统窗口菜单占用）。已改为运行期候选链注册，自动回退到 **Ctrl+Alt+Space** 成功，应用不再因快捷键冲突 panic。
2. **HUD 唤出已验证**：进程窗口枚举显示 HUD 初始 `visible=False`（656×104）；二次启动触发单实例回调后变为 `visible=True`，第二实例正常退出。
3. **托盘**：托盘图标由 1×1 透明占位改为真实图标（1×1 仅在解码失败时兜底），Windows 通知区域可见。
4. **窗口创建无异常**：HUD 与 Panel 仍在启动期按配置创建，未出现 WebView2 环境初始化失败。

未在本轮自动验证、需人工确认的交互项（快捷键注入与截图受自动化策略限制）：Tab 切 AI 态、Esc 收起 HUD、面板内的追问/取消。建议在 `npm run tauri dev` 下人工过一遍。

> 环境提示：WebView2 需要写入 `%LOCALAPPDATA%\com.magpie.assistant`，在受限沙箱中运行会得到 `ERROR_BUSY (0x800700AA)`。这是运行环境限制，不是应用缺陷；在正常桌面会话下直接运行或 `npm run tauri dev` 即可。
