---
status: 规划稿（待评审）
last_updated: 2026-11-23
---

# 喜鹊助手 · 实施计划

> 本版三条修订主线（依据用户 2026-11-23 的初始化要求）：
> 1. **开发环境更正**：实际开发环境是 **macOS**，不是旧稿默认前提里的 Windows。Windows 是**交付与验收目标**，不是日常开发机。
> 2. **文档整合**：把《资料备份.md》里的架构原始素材、《岛仔显示联动可行性》里可落地的结论，合并进本计划与 CONTEXT，避免同一结论散落多处。
> 3. **框架先行**：先把预留框架与接口搭起来，再一个功能一个功能实现；P2 能力只留接口位，不写实现。

## 1. 背景与目标

以火柴/火萤酱的悬浮入口为形态，做一个**Windows 优先交付、macOS 上开发**的桌面效率工具。核心是**一个输入框、四种去向**：本地文件/程序、计算器、翻译、AI 直答。

需求来源：[REQUIREMENTS.md](./REQUIREMENTS.md)（已批准，含 2026-11-23 环境表述修订）。术语定义：[CONTEXT.md](./CONTEXT.md)。架构原始素材：[资料备份.md](./资料备份.md)（已整合，保留作历史留档）。形态参考结论：[岛仔显示联动可行性_2026-11-12.md](./岛仔显示联动可行性_2026-11-12.md)（已整合可落地部分）。

**交付判定**：在 Windows 主机上完成七项 P0，通过验收表 11 条。

### 1.1 三条不可动摇的原则

| # | 原则 | 含义 |
|---|------|------|
| 1 | 开发环境是 macOS | 日常写码、跑前端、跑单测都在 macOS；Windows 只在需要实机验收时出现 |
| 2 | 先 Windows 产品，预留 macOS 可迁移性 | 先实现 Windows 上要交付的行为；平台差异全部隔离在 trait / 接口之后，同一份源码在 macOS 上可编译、可运行（能力可弱于 Windows），不为 macOS 单独分叉 |
| 3 | 框架先行，逐个实现 | 先搭出调度器骨架、各预留接口与 IPC 契约，再用计算器做第一条竖切冒烟；P2 能力只注册接口位，不写实现 |

## 2. 需求来源与文档地图

| 文档 | 角色 | 本计划如何整合 |
|---|---|---|
| REQUIREMENTS.md | 需求与交互定义（已批准） | P0/P1/P2、键盘映射、界面状态、验收标准直接沿用；修订其 Windows/macOS 表述 |
| CONTEXT.md | 术语表（纯术语，不含实现） | 统一术语；补入「两态展开」；修正关联文档指向 |
| 资料备份.md | 架构原始素材（早期草稿） | 调度器三件事、AI 搜索子代理工具位、MiniApp 框架、PeerService 留白、薄桥接层修正，全部合并进 §2/§3/§4；原件标记归档 |
| 岛仔显示联动可行性_2026-11-12.md | 形态参考分析 | 采纳「路 B 分区契约」与「路 A 视觉语言」，拒绝 fork 联动；常驻态按用户改判做「灵动岛」形态（可开关），结论落为 M0/M1 任务 |

## 3. 技术方案

### 3.1 架构设计

三个进程角色、两个窗口、一条类型化 IPC 契约。

```
┌─ Tauri 主进程 (Rust) ───────────────────────────────┐
│  hotkey      热键后端抽象                          │
│              ├ HookBackend      (Win, 双击 Ctrl)    │
│              ├ RegHotkeyBackend (Win, Alt+Space)    │
│              └ MacBackend       (mac, 开发用)       │
│  window      HUD / Panel 生命周期、多屏定位          │
│              └ avoidTop: 140pt（避让顶部信息岛，见 §3.7）│
│  search      SearchBackend trait                   │
│              ├ EverythingBackend (Win, FFI)        │
│              ├ StartMenuBackend  (两端通用)         │
│              └ MdfindBackend     (mac)             │
│  rank        结果评分与排序                          │
│  launch      打开 / 定位 / 复制路径                  │
│  gateway     DeepSeek 流式、Key 存取、取消令牌        │
│  auth        Identity trait（v1 只有本地身份）        │
│  peer        PeerService trait（v1 空实现，仅接口位） │
└──────────────────┬─────────────────────────────────┘
                   │ tauri-specta 生成的类型化 IPC
        ┌──────────┴───────────┐
        │                      │
┌─ HUD webview ──────┐  ┌─ Panel webview ─────────┐
│ 调度器状态机        │  │ 仅承载 AI 问答与追问      │
│ TriggerRegistry    │  │ 会话状态（独立持有）      │
│ MiniApp 注册表      │  │ 流式渲染 / 错误重试       │
│ 计算器 / 翻译 / 启动网格 / 结果列 │  │                        │
└────────────────────┘  └────────────────────────┘
```

关键边界（与火柴一致，不因 macOS 开发而改变）：

- **翻译全程在 HUD 内完成**：触发词 → Enter → 结果卡片出现在结果列表，不弹任何独立窗口。
- **只有 AI 搜索用独立持久窗口。** 面板只服务 AI 问答与追问，不承载翻译、计算器、本地结果。
- **HUD 与 Panel 不共享前端状态。** 二者是两个 JS 上下文。HUD 按 Enter 进 AI 时，把初始问题通过窗口启动参数交给 Panel，然后隐藏自己。
- **只有 Panel 持有会话。** v1 关闭即销毁，不落盘。
- **搜索与模型调用都在 Rust 侧。** 前端只发查询、收结果，不做网络请求，避免 CORS 与 Key 泄露。
- **平台判断只出现在 trait 实现里。** 前端不出现 `if (isWindows)` 这类分支（见 §3.5）。

### 3.2 技术栈（具体到依赖）

**脚手架**（一次性，在 macOS 上执行）：

```bash
npm create tauri-app@latest
# 选项：TypeScript / npm / React / TypeScript
cd magpie_bot && npm install
```

**版本与依赖清单**（安装时锁到当时最新稳定版，不追预发布）：

| 层 | 包 | 说明 |
|---|---|---|
| 桌面框架 | `tauri` 2.x（feature: `tray-icon`） | 主框架 |
| IPC 类型 | `tauri-specta` 2.x + `specta` 2.x | Rust→TS 类型自动生成；若仍是 RC，锁当时最新 RC |
| 权限声明 | `src-tauri/capabilities/*.json` | 每个窗口一份 capability，见 §3.7 |
| 序列化 | `serde` 1.x / `serde_json` 1.x | 配置与 IPC 结构 |
| 异步 | `tokio` 1.x（`sync`, `time`） | 取消令牌、超时 |
| HTTP | `reqwest` 0.12.x（`stream`, `json`, `rustls-tls`） | DeepSeek SSE |
| 流解析 | `futures-util` 0.3.x | 逐块读取响应体 |
| Key 存储 | `keyring` 3.x | **Windows Credential Manager / macOS Keychain** |
| 动态加载 | `libloading` 0.8.x | 加载 Everything64.dll（仅 Windows） |
| Win32 | `windows` 0.58+（`WindowsAndMessaging`, `KeyboardAndMouse`, `LibraryLoader`） | 钩子、窗口层级；**仅 `cfg(target_os = "windows")`** |
| 错误 | `thiserror` 2.x | 各模块错误枚举 |
| 日志 | `tracing` 0.1.x + `tracing-subscriber` 0.3.x（`env-filter`） | 耗时埋点 |
| 前端 | `react` 19.x / `react-dom` 19.x | 两个窗口共用组件 |
| 构建 | `vite` 最新 + `@vitejs/plugin-react` | 多页面入口 |
| 类型 | `typescript` 5.x | — |
| 测试 | `vitest` 最新 | 只测纯逻辑 |
| Tauri API | `@tauri-apps/api` + `@tauri-apps/cli` 2.x | — |
| 单实例 | `tauri-plugin-single-instance` | 官方插件 |

**明确不引入**：Tailwind / 任何 UI 组件库（两个窗口样式量小，原生 CSS + 变量足够）、XState（状态只有五个）、状态管理库（用 `useReducer`）、Electron。

**多窗口开发配置**：Vite 两个 HTML 入口 `index.html`（HUD）与 `panel.html`（Panel）；`tauri.conf.json` 里两个 window 分别指向 `index.html`、`panel.html`，label 分别为 `hud` 与 `panel`。

### 3.3 数据模型

```ts
type ResultKind = "app" | "folder" | "file" | "calc" | "translate" | "number"; // number 为来电归属预留位

interface SearchResult {
  id: string;
  kind: ResultKind;
  name: string;
  path?: string;
  score: number;
  source: "everything" | "startmenu" | "mdfind" | "miniapp";
}

interface Query { text: string; limit: number }

interface ChatMessage {
  role: "user" | "assistant";
  content: string;
  status?: "streaming" | "done" | "aborted" | "error";
}

interface AppSettings {
  hotkeyBackend: "hook" | "reghotkey" | "mac";
  theme: "light" | "dark" | "system";
  maxResults: number;
  translateTriggers: string[];
  launcherPins: string[];
  clipboardPrefillOnWake: boolean; // 默认 false；号码功能预留，见 REQUIREMENTS §10.1
  islandEnabled: boolean; // 常驻灵动岛态开关，默认 false；见 §3.7 / ADR-7
}
```

> `number`、`clipboardPrefillOnWake`、`islandEnabled` 只是**接口位/设置位**；常驻灵动岛态按框架先行原则后续单独实现（原则 3、ADR-7）。

### 3.4 目录结构

```
仓库根/
├── README.md                 # 文档索引（见本仓库 README）
├── LICENSE
└── 火柴复现工程/              # Tauri 工程根（文档与代码同处此目录）
    ├── CONTEXT.md / REQUIREMENTS.md / PLAN.md
    ├── 资料备份.md             # 架构原始素材，已归档
    ├── 岛仔显示联动可行性_2026-11-12.md
    ├── package.json
    ├── index.html              # HUD 入口
    ├── panel.html              # 会话面板入口
    ├── src/
    │   ├── hud/                # 悬浮框窗口
    │   │   ├── main.tsx
    │   │   ├── Hud.tsx
    │   │   └── components/     # 结果列表、启动器网格、计算卡片、翻译卡片
    │   ├── panel/              # 会话面板窗口（仅 AI）
    │   │   ├── main.tsx
    │   │   ├── Panel.tsx
    │   │   └── components/     # 消息气泡、追问输入、错误重试
    │   ├── core/
    │   │   ├── scheduler/      # 状态机（纯函数，重点单测对象）
    │   │   ├── miniapps/       # TriggerRegistry + 注册表 + calculator（首个竖切）
    │   │   ├── rank-client/    # 评分层的前端侧调用与展示截断
    │   │   └── ipc/            # tauri-specta 生成物 + 唯一的 invoke 封装（勿手改生成部分）
    │   └── shared/             # CSS 变量、键盘工具、通用组件
    ├── src-tauri/
    │   ├── src/
    │   │   ├── main.rs / lib.rs
    │   │   ├── hotkey/         # mod.rs + hook_win.rs + reghotkey_win.rs + mac.rs
    │   │   ├── window.rs       # 两窗口生命周期、多屏定位、avoidTop 常量
    │   │   ├── search/         # mod.rs + everything_win.rs + startmenu.rs + mdfind_mac.rs
    │   │   ├── rank.rs         # 评分与排序（纯函数，单测对象）
    │   │   ├── launch.rs
    │   │   ├── gateway/        # deepseek.rs + secret.rs + cancel.rs
    │   │   ├── auth.rs
    │   │   ├── peer.rs         # PeerService trait，空实现
    │   │   ├── settings.rs
    │   │   └── logging.rs
    │   ├── capabilities/       # hud.json / panel.json（权限声明，见 §3.7）
    │   ├── vendor/Everything64.dll   # MIT，随包分发（仅 Windows 包）
    │   └── tauri.conf.json
    └── tests/                  # vitest：调度器、表达式、评分
```

> 工程根目录暂沿用 `火柴复现工程/`（减少搬运）；是否迁到仓库根列入 §12 开放问题。

### 3.5 开发环境与跨平台约定（本次重点修订）

**实际开发环境是 macOS，Windows 是交付/验收目标机。** 旧稿「两台机器都是完整开发机」「模式 B·Windows 主开发」的表述已作废。

| 机器 | 角色 | 需要 |
|---|---|---|
| **macOS** | **唯一日常开发机**：写码、跑前端、跑单测、`cargo check`（跨平台模块） | Node 20 LTS+、Rust stable（`rustup`）、Xcode Command Line Tools |
| **Windows** | **交付与验收目标机**：实机验收钩子、多屏、DPI、Everything、打包 | Node 20 LTS+、Rust stable（**MSVC toolchain**）、Visual Studio 2022 Build Tools（勾选"使用 C++ 的桌面开发"）、WebView2 Runtime（Win11 自带）、Everything 安装并常驻 |
| 两端 | 同一 git 远端账号，`git config user.name/email` 一致 | — |

开发循环：

| 步 | 位置 | 动作 | 产出 |
|---|---|---|---|
| 1 | macOS | 认领一个里程碑或子任务 | 一张任务卡 |
| 2 | macOS | 写代码，`npm run tauri dev` 跑 HUD/Panel | 本地可交互 |
| 3 | macOS | `npm run test` + `cargo test` + 基础 UI 手测 | 单测绿、布局不炸 |
| 4 | macOS | commit + push | 远端更新 |
| 5 | Windows | `git pull && npm ci && cargo check`，必要时 `npm run tauri build -- --debug` 实机验收 | 编译通过 + 真实程序 |
| 6 | 任一端 | 修问题，回到步 2 | — |

**诚实说明**：macOS 上 `cargo check` 只覆盖非平台专属模块；Windows 专属代码（`cfg(windows)`）在 macOS 上会被编译期排除，无法靠它验证。因此 Windows 专属代码必须**少而集中**（仅 §3.6 列出的文件），由 Windows 验收机兜底编译。不追求"在 macOS 上验证 Windows 专属逻辑"，而是靠隔离把风险压到可人工审查的范围。

**跨平台编码约定**：

| 项 | 规则 |
|---|---|
| 路径 | Rust 一律用 `PathBuf` / `std::path`；前端用正斜杠；禁止硬编码 `C:\` 或 `/Users/` |
| 平台判断 | 只在 Rust 的 trait 实现里分流；前端不允许出现 `if (isWindows)` 这类分支 |
| 脚本 | `package.json` 只用 Node 脚本（`node scripts/xxx.mjs`），不用 `rm -rf`、`cp` 等 POSIX 命令；需要时用 `rimraf`、`cross-env` |
| 换行 | 根目录 `.gitattributes` 固定 `* text=auto eol=lf`，避免两端 diff 噪音 |
| 文件名大小写 | 两端都不敏感，但 git 内改大小写会出错；改名统一用 `git mv` |
| 视觉验证 | **以 Windows 为准**（交付平台）；macOS 只做布局不炸的检查 |
| 依赖探测 | macOS 上没有 Everything，Windows 上没有 mdfind；两者都必须能在缺失时优雅降级 |

**平台隔离纪律**：Windows 专属代码只出现在 `hotkey/hook_win.rs`、`reghotkey_win.rs`、`search/everything_win.rs`、`window.rs` 的 `cfg(windows)` 分支。其他模块必须两端都能编译。macOS 侧用 `mdfind`（`std::process::Command` 调用）与 `MacBackend` 热键后端，保证"能跑完整链路"，不追求与 Windows 等价的能力。

**坚果云忽略规则**（必须设置，否则同步会拖垮机器）：

```
.git/
node_modules/
src-tauri/target/
dist/
*.log
```

### 3.6 预留框架清单（框架先行的落点）

这一节是"框架先行、逐个实现"的具体清单。**v1 只搭骨架与注册位，不写 P2 实现。**

| 框架 / 接口 | 位置 | v1 状态 | 扩展点 |
|---|---|---|---|
| 搜索调度器状态机 | `src/core/scheduler/` | **实现**：五态状态机 + reducer，纯函数重点单测 | 新增模式只加状态，不改 UI |
| TriggerRegistry | `src/core/miniapps/` | **实现**：触发词→处理器的注册与剥离 | 新增小程序只注册一条 |
| MiniApp（统一输入/输出契约） | `src/core/miniapps/` | **实现骨架**：calculator 为首个竖切；translate 接入网关 | Wox 式宿主隔离为未来选项，本版进程内实现，路由层不动 |
| SearchBackend trait | `src-tauri/search/` | **实现**：Everything(Win) / StartMenu / mdfind(mac) | 换后端只改实现，不动调度器 |
| 模型网关 | `src-tauri/gateway/` | **实现**：DeepSeek 流式 + Key + 取消 + 额度检查点 | 翻译复用；商业化在额度检查点接入 |
| AI 搜索子代理工具位 | 网关内 v2 位 | **仅占位**：v1 注册零个 Search/Fetch 工具 | v2 暴露工具；硬上限"最多搜 3 次、抓 2 页"防过度调用 |
| 身份接口 Identity | `src-tauri/auth.rs` | **仅占位**：v1 只有"本机本人"实现，不做登录 UI | 将来权限/商业化挂接 |
| PeerService | `src-tauri/peer.rs` | **仅占位**：空实现，返回"未启用" | 将来作为独立后台服务接入，不侵入调度器 |
| 来电归属号码小程序 | 注册表 + 调度器判定点 | **仅占位**：触发词位 + "整串是否号码"判定点 | 号段库来源/许可待定（REQUIREMENTS §10.1） |
| 单位转换/数字转大写/二维码 | 注册表占位 | **仅占位** | 后续逐个实现，路由层不改 |
| 皮肤系统 | CSS 变量 | **明确不做**文件格式/编辑器；只做明暗两套变量 | 切换即换一组变量值 |
| 两态展开视觉语言 | HUD 前端 CSS/动效 | **M1 采纳**：借岛仔的展开节奏 | 见 §3.7 与 §6 |
| 常驻灵动岛态 | 独立窗口 + 托盘 + 设置 | **先留框架位**：`islandEnabled` 设置位与窗口骨架，功能后续单独实现，不进 v1 验收 | 见 §3.7 / ADR-7 |
| IPC 契约 | `src/core/ipc/` 生成物 | **实现**：tauri-specta 一份定义两端同步 | Rust 改类型，TS 编译期发现漂移 |

**薄桥接层的正确表述**（整合自《资料备份.md》）：不是"把 Rust 限制在薄桥接层"，而是**"把 Rust 的厚度集中在少数几个接口极简的系统能力模块（搜索后端、模型网关、安全存储），其余调度、路由、小程序、UI 全部放 TS"**。桥接层按领域聚合 command（`search` / `launch` / `system`），不为每个操作开一个 command，避免类型漂移。

### 3.7 HUD 定位与形态参考（整合自岛仔分析）

采纳《岛仔显示联动可行性》里的分区契约与视觉语言；原分析建议"不做常驻收起态"，**用户 2026-11-23 已拍板改为做常驻灵动岛态**（见 ADR-7），但仍坚持只有一个唤出输入框、不 fork 岛仔：

| 决策 | 内容 | 落点 |
|---|---|---|
| **采纳·路 B 分区契约** | 喜鹊 HUD 保持屏幕中心偏上唤出，顶部留安全距 `avoidTop = 140pt`，避免遮住顶部信息岛 | M0：`window.rs` 加常量 |
| **采纳·路 A 视觉语言** | 从顶部细横条展开成输入框（150–200ms、带回弹），卡片向下展开 | M1：HUD 两阶段渲染 + CSS 动效；须用中文输入法实测焦点不丢 |
| **采纳（用户改判）·常驻灵动岛态** | 做成类似 iPhone 灵动岛的形态；**在设置中可开/可关**（默认关）；独立于 HUD 的按需唤出，仅作为信息展示入口。按框架先行原则先留接口位与窗口骨架，功能后续单独实现，不进 v1 验收 | M0：`islandEnabled` 设置位；窗口/托盘骨架预留；ADR-7 |
| **不采纳·进程联动（fork 岛仔）** | 岛仔无插件/HTTP/管道入口，fork 维护成本极高且收益极小 | 不写代码；如仍想要，先去上游开 Issue 问"外部状态槽" |

**分工约定（已拍板，2026-11-23）**：岛仔管"被动地看"，喜鹊管"主动地找"；**喜鹊不做 AI 额度/待办/音乐，岛仔不做搜索**。常驻灵动岛态只承载喜鹊自身的状态展示（如"本地结果就绪"提示），不复制岛仔的信息面。

### 3.8 日常命令

| 目的 | 命令 | 机器 |
|---|---|---|
| 前端单测 | `npm run test` | macOS |
| Rust 单测 | `cargo test --manifest-path src-tauri/Cargo.toml` | macOS |
| 开发模式 | `npm run tauri dev` | macOS |
| 快检（不打包） | `cargo check`（在 `src-tauri/` 下） | macOS（验跨平台模块）/ Windows（验全量） |
| Windows 可运行构建 | `npm run tauri build -- --debug` | Windows |
| Windows 发布构建 | `npm run tauri build` | Windows |
| 生成 IPC 类型 | `cargo test export_bindings` | macOS |

### 3.9 git 约定

- 主干 `main` 始终可编译；开发用短分支 `m2-search` 这类"里程碑号-主题"。
- 提交信息用 Conventional Commits，正文中文：`feat(search): 接入 Everything IPC 查询`。
- 每个里程碑关闭时打 tag：`m0`、`m1`……
- Windows 验收机只做兼容性修正，改完立即 push 回 `main`，不在 Windows 上长期分叉。

### 3.10 里程碑门禁（"引导式执行"的具体含义）

每个里程碑同时满足四项才算关闭，缺一项不进入下一阶段：

1. **可演示**：在 macOS 或 Windows 上跑起来，留截图或 30 秒录屏
2. **单测绿**：`npm run test` 与 `cargo test` 全部通过
3. **对端编译通过**：Windows 验收机上 `cargo check` 无错（macOS 为开发机时验 Windows 专属代码）
4. **实测数字**：验收表对应行填入真实测量值

关闭后我会停下来，把四项证据摆给你，你确认再开下一个。

### 3.11 Tauri 2 权限声明规范（capabilities）

**这一节是硬规则，不是建议。** Tauri 2 的 ACL 决定窗口能访问哪些命令；一个窗口如果没有匹配任何 capability，它就**完全无法访问 IPC 层**。典型症状正是"Rust command 写对了、前端 invoke 也写对了，但功能就是不工作"。这类问题在编译期不报错，只在运行时失败。

**固定结构**：一个窗口一份 capability，新增窗口必须同步新增文件。

`src-tauri/capabilities/hud.json`：

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "hud",
  "description": "悬浮框窗口的 IPC 权限",
  "windows": ["hud"],
  "permissions": ["core:default"]
}
```

`src-tauri/capabilities/panel.json`：

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "panel",
  "description": "会话面板窗口的 IPC 权限",
  "windows": ["panel"],
  "permissions": ["core:default"]
}
```

**新增一个 Rust command 时必须一次做完五件事**（AI 生成代码时按此清单执行，不许只做前两步）：

1. 在 Rust 侧定义 `#[tauri::command]` 并加入 `generate_handler!`
2. 跑 `cargo test export_bindings`，让 tauri-specta 生成 TS 类型
3. **同步更新对应窗口的 capability 文件**：窗口 label 必须出现在 `windows` 数组里；涉及核心或插件能力时补 `core:*:allow-*` 或 `plugin:*:allow-*` 权限项
4. 前端只通过 `src/core/ipc/` 的统一封装调用，不散落裸 `invoke`
5. 在目标窗口实机冒烟一次，确认不是"编译通过但运行被拒"

**新增窗口时**：先写 capability，再写窗口；顺序反过来很容易漏。

**结论要回写文档**：应用自有 command 在当前 Tauri 版本下是否需要在 capability 中显式声明，M0 用一个最小 command 实测确认，把结果补进本节。文档里不写死未经验证的结论。

### 3.12 禁止事项

- 不在坚果云同步目录里跑完整构建（`target/` 会产出数 GB 并参与同步）
- 不在两台机器同时打开同一工程目录
- 不在 Windows 上重构代码，只修兼容问题
- 不把 API Key 写进任何文件、提交历史或日志
- 不手改 `src/core/ipc/` 下的生成文件
- **不新增 command 而不更新 capability；不新增窗口而不写 capability**
- 不写只能在单一平台跑的脚本或硬编码绝对路径
- 不在未提交改动的情况下切换开发机（Mac 为日常机，Windows 只做验收）

## 4. 任务拆分（框架先行，逐个实现）

| # | 里程碑 | 内容 | 依赖 | macOS 上验证 | Windows 验收 | 复杂度 |
|---|---|---|---|---|---|---|
| M0 | 骨架与框架先行 | Tauri 双窗口、capabilities 基线、单实例、托盘、热键后端抽象（含 mac 后端）、设置读写、**调度器空壳与 TriggerRegistry 骨架**、**SearchBackend / gateway / auth / peer 接口定义**、**avoidTop=140pt 常量**、**islandEnabled 设置位与常驻岛窗口骨架（仅结构，无功能）** | 无 | 双击 Ctrl（mac 后端）唤出空 HUD；框架接口编译通过 | 两套热键后端可唤出；钩子是否被安全软件拦；command 在 hud/panel 中均可 invoke | 中 |
| M1 | 调度器与纯本地能力 | 状态机五态互转、启动器网格、计算器卡片（**第一条竖切**）、IME 组词处理、**两态展开动效** | M0 | 五态互转；`2+2` 出卡片、`2+2会议纪要` 不出；展开动效不丢焦点 | 中文输入法下 Tab/Ctrl+Tab 正常 | 中 |
| M2 | 本地搜索 | Everything FFI、程序搜索、评分层、打开/定位/复制、降级 | M1 | mdfind 跑通同一接口 | 前 4 字母目标进前 3；未运行 Everything 时降级可用 | 高 |
| M3 | 会话面板与 AI | 独立窗口、DeepSeek 流式、追问、取消、错误重试 | M1 | 面板可拖动、Esc 关闭回焦 | 首字延迟实测；断网错误态 | 高 |
| M4 | 翻译 | 触发词、语言自动识别、**全程 HUD 内呈现** | M1 | `fy hello` 在悬浮框出中文结果 | — | 低 |
| M5 | 主题与 mac 搜索 | 明暗变量、设置页、mdfind 完善 | M2 | 主题切换无闪烁 | 设置项持久化 | 低 |
| M6 | 全量验证 | 逐条验收、阈值校准、打包 | M2–M5 | — | 验收表 11 条 | 中 |

**顺序理由**：

- **M0 先搭框架**：按用户要求"先把框架搭好"，本里程碑不交付任何具体功能，只交付骨架、接口与 IPC 契约；用计算器之前先让整个空壳能编译、能唤出。
- **M1 用计算器做第一条竖切**：计算器是纯本地、零依赖的最小功能，用它验证"触发词→调度器→卡片"的全链路，证明框架可用，再往里填搜索/AI/翻译。
- **M2 在 M3 之前**：本地搜索一旦可用，工具从第一天就能日常使用；先做 AI 会得到一个每天只用一次的玩具。M2 若卡住，退路是先交评分层 + 程序搜索，文件搜索后补。
- **P2 功能不排在里程碑表**：它们只有接口位，等 v1 交付后按 §4 的注册表逐个实现。

## 5. 风险与对策

| 风险 | 影响 | 对策 |
|---|---|---|
| **capability 漏配** | 命令调用运行期被拒，编译期无感 | §3.11 五步清单；新增窗口先写 capability；M0 实机冒烟 |
| 跨平台写法不一致 | 一端能跑、另一端编译或运行失败 | §3.5 编码约定；Windows 专属代码只集中在 §3.6 列出的文件，Windows 验收机兜底编译 |
| 低级键盘钩子被安全软件拦截 | 主唤出方式失效 | 双后端，一键切 Alt+Space；设置页显示当前生效后端 |
| 钩子丢键或误触 | 复制粘贴时误弹 | 两次 Ctrl 间出现其他键即作废；间隔阈值可调 |
| 中文输入法吞 Tab / 触发空搜索 | 用户以为功能坏了 | Tab 组词中让给 IME；本地搜索以 composition 结束文本为准；两态展开动效须实测焦点 |
| Everything 未运行 | 本地模式失效 | 程序搜索兜底 + 明确提示，不做静默失败 |
| 两个 webview 状态漂移 | 面板与 HUD 显示不一致 | 不共享状态；会话只属于 Panel |
| WebView2 内存超预期 | 验收不达标 | M6 实测后重定阈值，不拿拍脑袋的数字验收 |
| 双机同步冲突 | 工程损坏 | git 远端同步源码；坚果云忽略 `.git/`、`node_modules/`、`target/` |
| DeepSeek 限流或断网 | AI 功能不可用 | 面板内错误态 + 重试；请求带超时与取消 |
| **框架搭过头变空转** | 迟迟没有可演示功能 | M0 严格限时，只搭接口与空壳；M1 立即用计算器做竖切验证框架 |

## 6. 验收标准

沿用 [REQUIREMENTS.md](./REQUIREMENTS.md) 第 5 节 11 条。M6 逐条记录实测值，不达标项写入遗留清单。

## 7. 执行模式

- [x] 引导式：每个里程碑完成后暂停，用 §3.10 的四项证据确认后继续
- [ ] 自动式
- [ ] 验证驱动

理由：跨机器验证无法自动化，且 M2 的排序准不准、M3 的流式顺不顺，只有你亲自用才知道。

## 8. 多视角分析摘要

**架构视角**：Rust 侧三个厚模块（搜索、网关、热键）都是"内部复杂、对外一个函数"，前端只做编排与渲染。翻译不单独开窗，复用 HUD 的结果卡片通道。预留框架（§3.6）先于功能实现。

**实现视角**：M2 是唯一的硬骨头——Everything FFI、评分层、降级路径三者叠加。M0 的热键与 capabilities 是两个前置坑，都有明确清单可依；M1 的计算器竖切是验证框架是否好用的第一块试金石。

**验证视角**：自动化测试只覆盖纯逻辑（调度器状态机、表达式解析、评分函数）；其余靠人工验收并记录数字。

**安全视角**：Key 进系统凭据库（Win Credential Manager / macOS Keychain）；前端永不接触 Key；日志脱敏。capability 按窗口最小授权，面板拿不到本地搜索以外的能力。

## 9. 跨切面关注点

| 关注点 | 状态 | 说明 |
|---|---|---|
| 安全/合规 | Ready | Key 不落前端；日志不记输入内容；capability 最小授权 |
| 性能/效率 | Needs attention | 内存阈值待实测；Everything 查询需在工作线程，不阻塞 UI |
| 可访问性/可用性 | Ready | 全键盘闭环；高对比主题；不用纯色块表达状态 |
| 可观测性/可维护性 | Needs attention | 需要耗时埋点，否则验收标准无法测量 |
| 可迁移性（新增） | Ready | 平台差异隔离在 trait 后；macOS 开发、Windows 交付；同源码双端可编译 |

## 10. 开放问题与决策

### 待验证（不阻塞开工）

- Tauri 2 当前版本下，应用自有 command 是否需在 capability 中逐条声明——M0 实测后回写 §3.11
- 双击 Ctrl 的间隔阈值取 400ms 是否顺手，Windows 实测后调
- DeepSeek 是否换 `deepseek-reasoner`，取决于首字延迟
- Everything 结果上限（初定拉 50 条进评分层，展示截断到 20）
- 工程根目录是否沿用 `火柴复现工程/`（建议沿用，减少搬运）

### 已拍板的决策（2026-11-23）

1. **常驻态要做成类似 iPhone 灵动岛的形态，设置中可开/可关**（默认关）。原分析建议不做，用户改判；按框架先行原则先留接口位与窗口骨架，功能后续单独实现（ADR-7）。
2. **功能重叠区按建议划分**：岛仔管"被动地看"，喜鹊管"主动地找"；喜鹊不做 AI 额度/待办/音乐，岛仔不做搜索（ADR-6）。

### 已定的默认值（可推翻）

| 项 | 默认值 |
|---|---|
| 翻译触发词 | `fy`、`翻译` |
| AI 请求超时 | 首字节 10s，整段 60s |
| 日志内容 | 仅耗时、状态码、错误类型 |
| 面板层级 | 不置顶，失焦后保持打开 |
| HUD 顶部避让 | 140pt（avoidTop） |
| 常驻灵动岛态 | 关闭（`islandEnabled = false`），设置中可开启 |

### 架构决策记录（ADR）

**ADR-1：双击 Ctrl 用低级键盘钩子，同时保留 RegisterHotKey 后端**
难以逆转（改动热键层要动窗口生命周期）；两个方案各有明确取舍。后果：多写约三十行，换来杀软环境下的可用性。

**ADR-2：只有 AI 使用独立窗口，翻译与计算在 HUD 内完成**
难以逆转（窗口拓扑决定状态管理和 IPC 设计）；把翻译也放进独立窗口会让高频小工具多一步焦点迁移。后果：HUD 需要承载结果卡片的多形态，面板保持单一职责。

**ADR-3：本地搜索用 Everything，对外封成单一查询接口**
可逆但代价高（换后端要重做排序与降级）。后果：MIT 许可不阻碍商业化，但用户需安装 Everything；macOS 侧用 mdfind 走同一接口。

**ADR-4：一个窗口一份 capability，新增命令与权限声明同批产出**
难以逆转（改权限模型要重做窗口授权边界）；不记录理由会让后人以为 ACL 配置是冗余。后果：每次新增 command 多一步，换来运行期不静默失败。

**ADR-5：框架先行，P2 只留接口位**
难以逆转（决定模块边界与注册表设计）；用户明确要求"不要着急完成具体产品"。后果：M0 交付骨架而非功能，用 M1 计算器竖切验证框架后再逐个实现。

**ADR-6：macOS 为开发机、Windows 为交付目标；与岛仔功能分工按"被动看/主动找"**
平台差异隔离在 trait 后；功能重叠区已拍板：喜鹊不做 AI 额度/待办/音乐，岛仔不做搜索。后果：Windows 专属代码少而集中，由验收机兜底编译；两工具信息不重复。

**ADR-7：常驻灵动岛态可开关，先留框架位后实现**
用户 2026-11-23 改判：原分析建议不做常驻收起态，但用户明确要做成类似 iPhone 灵动岛的形态，并在设置中可开/可关（默认关）。它独立于按需唤出的 HUD，仅承载喜鹊自身状态展示，不复制岛仔的信息面；不 fork 岛仔。后果：M0 增加 `islandEnabled` 设置位与窗口/托盘骨架，功能实现排在 v1 验收之后，避免框架搭好后急于堆功能。

## 11. 战略考量

**我们在解决正确的问题吗？** 是。四个场景都是高频、低价值动作，合并到一个入口的收益明确。但这类工具的价值来自**肌肉记忆**，不是功能数量。v1 只要三周内你每天真的按双击 Ctrl，就算成功；装了不用，加再多功能也只能躺托盘。

**什么可能让这个方法失败？** 三件事：钩子在你的安全软件下失效；Everything 排序不如预期，导致你仍去开资源管理器；AI 直答在时效性问题上答错，让你失去信任。前两个有对策，第三个目前没有退路，只能靠换问法。

**需求可能如何变化？** 最可能是用起来之后你发现 AI 面板比本地搜索更重要，那时联网搜索会从 P2 提前。当前架构已为此留了工具位（§3.6），不需要重构。

## 12. 阶段需求

| 阶段 | 是否必需 | 理由 |
|---|---|---|
| 准备/环境 | Yes | 两端工具链、坚果云忽略规则、git 远端（§3.5） |
| 设计/规划 | Yes | 已完成（本文档 + 已整合的素材） |
| 实现/执行 | Yes | M0–M6，框架先行、逐个实现 |
| 验证/测试 | Yes | 单测覆盖纯逻辑 + Windows 人工验收 |
