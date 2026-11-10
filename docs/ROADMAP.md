# RustPing 开发路线图

仿 lolPing 的通用桌面 Ping 轮盘：按住热键拖拽唤出轮盘，松开在屏幕任意位置打出标记。
原则：**只保留关键功能，性能优先，GPU 加速**。项目定位见 [AGENTS.md](../AGENTS.md)。

## 技术选型（2026-11-08 确认）

| 项 | 选择 | 说明 |
|---|---|---|
| GUI/渲染 | egui + wgpu | 纯 Rust、GPU 加速；扇形/圆环/文字用 egui 图元（Mesh/Circle/Text）足够，不手写着色器；覆盖层与设置窗口共用一套栈 |
| 窗口 | winit（直连） | 透明、置顶、无边框、多显示器枚举；egui-winit + egui-wgpu 手动接线。原定 eframe，阶段 1 发现其单主窗口假设与每显示器覆盖窗口冲突，已移除（2026-11-10） |
| 平台层 | windows-rs | 低级输入钩子、窗口扩展样式、托盘、注册表 |
| 托盘 | tray-icon | 独立 crate，成熟稳定 |
| 配置 | serde + toml + notify | TOML 配置 + 文件变更热重载 |
| 日志 | tracing | 关键路径耗时打点（钩子→首帧延迟等） |

### 选型简介与文档

**GUI / 渲染层**

- **[egui](https://github.com/emilk/egui)**（[官网](https://www.egui.rs/) / [文档](https://docs.rs/egui)）
  纯 Rust 的即时模式（immediate mode）GUI 库：每帧用普通 Rust 代码描述整个 UI，无需手动同步控件状态。自带布局、字体与常用控件；`Shape`/`Mesh` 图元可画任意三角形，本项目的扇形轮盘、圆环动画、设置窗口都用它绘制。
- **[wgpu](https://wgpu.rs/)**（[文档](https://docs.rs/wgpu)）
  Rust 对 WebGPU 标准的跨平台实现，底层映射到 Vulkan / Metal / DirectX 12。GPU 加速渲染的入口，egui 的 wgpu 渲染后端由它驱动。
- **[eframe](https://github.com/emilk/egui)**（[文档](https://docs.rs/eframe)）
  egui 官方的桌面应用框架，把 winit（窗口 + 事件循环）和 wgpu（渲染后端）打包成一个 `run_native` 入口。本项目直接以它为应用骨架。
- **[winit](https://github.com/rust-windowing/winit)**（[文档](https://docs.rs/winit)）
  跨平台窗口创建与输入事件循环库，负责透明窗口、置顶、多显示器枚举、DPI 缩放等；是 egui / wgpu 生态共用的事件基础。
- **[raw-window-handle](https://github.com/rust-windowing/raw-window-handle)**（[文档](https://docs.rs/raw-window-handle)）
  从 winit 窗口取出平台原生句柄（Windows 上即 HWND）的互操作层。设置 click-through 等扩展窗口样式时，需要拿 HWND 去调 Win32 API。

**平台层（Windows）**

- **[windows-rs](https://github.com/microsoft/windows-rs)**（[文档](https://microsoft.github.io/windows-docs-rs/)）
  微软官方维护的 Windows API Rust 绑定，覆盖 Win32 与 COM。输入钩子（`SetWindowsHookExW`）、窗口扩展样式（`SetWindowLongPtrW`）、注册表开机自启等系统层调用全靠它。
- **[tray-icon](https://github.com/tauri-apps/tray-icon)**（[文档](https://docs.rs/tray-icon)）
  系统托盘图标与右键菜单库（tauri-apps 出品，纯 Rust、无 WebView 依赖）。用于托盘菜单「设置 / 暂停 / 退出」。

**配置与可观测性**

- **[serde](https://serde.rs/)**（[文档](https://docs.rs/serde)）
  Rust 事实标准的序列化框架，结构体加 derive 宏即可与各种格式互转。配置文件读写的核心。
- **[toml](https://github.com/toml-rs/toml)**（[文档](https://docs.rs/toml)）
  TOML 格式的解析/生成库，serde 兼容。本项目配置选 TOML 是因为手写友好、支持注释。
- **[notify](https://github.com/notify-rs/notify)**（[文档](https://docs.rs/notify)）
  跨平台文件系统变更监听（封装各 OS 原生 API）。实现 `config.toml` 保存后免重启热重载。
- **[tracing](https://github.com/tokio-rs/tracing)**（[文档](https://docs.rs/tracing)）
  tokio 团队的结构化日志与追踪库，支持 span 层级与耗时打点。用于验证"热键按下→轮盘可见 <50ms"这类性能指标。

## 总体顺序

**先啃最高风险的平台层，尽快打通核心闭环，配置与打磨靠后：**

```
M0 骨架 → M1 平台层(覆盖层+全局手势) → M2 核心闭环(轮盘→ping 动画)
        → M3 可用(配置+托盘+设置窗口) → M4 打磨与 v0.1.0 发布
```

理由：透明置顶穿透覆盖层 + 低级输入钩子是本项目**成败所系且风险最高**的部分
（窗口样式、焦点、全屏、DPI 都可能踩坑），第 1 天就要验证可行；轮盘渲染与
ping 动画是核心价值；配置/托盘/设置是易做的增量；发布打磨最后统一做。

每阶段的执行节奏、调试手段、保险机制与冒烟清单见 [DEVELOPMENT.md](DEVELOPMENT.md)。

## 阶段明细

### 阶段 0 — 工程骨架（M0，小）

- Cargo 项目 + 模块划分（见下文"模块布局"），`cargo clippy`/`cargo test` 全绿。
- GitHub Actions CI（windows-latest：fmt + clippy + test + build）。
- eframe 空窗口跑通（wgpu 后端确认可用）。

**验收**：`cargo run` 出空窗口；CI 绿。

### 阶段 1 — 覆盖层窗口基础（M1a，中）

- 每显示器一个无边框窗口：透明、置顶、不进任务栏、不抢焦点
  （Win32：`WS_EX_LAYERED`/`WS_EX_TOOLWINDOW`/`WS_EX_NOACTIVATE`）。
- click-through 可切换：`WS_EX_TRANSPARENT` 经 `SetWindowLongPtrW(GWL_EXSTYLE)`。
- 多显示器枚举与 PerMonitorV2 DPI 坐标换算（winit scale factor）。
- **按需渲染**：仅轮盘可见或有存活动画时请求重绘，空闲 CPU ≈ 0%。
- manifest：`dpiAware=PerMonitorV2`、`requestedExecutionLevel=asInvoker`。

**验收**：透明覆盖层铺满每块显示器，渲染一个彩色图形，穿透点击可点到下层窗口；
不抢焦点、不进 Alt-Tab/任务栏。

### 阶段 2 — 全局输入捕获（M1b，中）

- `WH_KEYBOARD_LL` 钩子监听热键按下/抬起（默认 Alt，可配置）；
  `WH_MOUSE_LL` 钩子在手势期间捕获移动/按键并**吞掉事件**（阻止 Alt 激活菜单、
  阻止下层窗口收到点击）。
- 钩子回调在安装线程执行 → 独立钩子线程 + 消息泵，与 UI 线程用 channel 通信，
  回调内逻辑极轻。
- 手势状态机（纯逻辑、可单测）：
  `Idle → Armed(热键按下) → WheelActive(拖动超阈值) → Selected(松开)`
  / `Cancelled(Esc、未过阈值松开)`。
- 已知限制记录在案：全屏独占游戏不可见（无边框窗口可用，lolPing 同限）；
  管理员权限进程的输入钩不到非管理员应用（文档提示可管理员运行）。

**验收**：任意应用上按住热键拖动/松开，日志输出完整状态机轨迹；
手势期间下层应用收不到鼠标事件、Alt 不触发菜单。

### 阶段 3 — 轮盘渲染与选中（M2a，中大）

- 轮盘几何（纯逻辑、可单测）：N 个扇形、中心死区、半径；角度命中测试。
- egui 绘制：扇形（三角剖分 Mesh）、hover 高亮、扇区中线上放图标/短文字、
  中心显示当前选中名。
- 动画：出现时缩放+淡入 ease-out（约 120ms），hover 渐变过渡。
- 轮盘跟随光标出现；松开时输出选中项 + 屏幕坐标。

**验收**：按住热键拖出轮盘，hover 有高亮反馈，松开日志打印选中项与坐标。

### 阶段 4 — Ping 标记动画（M2b，中）

- 标记实体（纯逻辑、可单测）：`{位置, 类型, 出生时间}`；动画约 1.8s：
  圆环扩散 + 淡出 + 图标弹出；并发标记上限（如 32）。
- 复用每显示器覆盖层单窗口绘制全部存活标记（不搞一窗一标记）。
- 与阶段 2 状态机对接：松开选中 → 在松开坐标生成标记。

**验收**：**核心闭环达成**——热键拖拽 → 轮盘 → 松开 → 屏幕该位置播放 ping
动画后消失；可连续打出多个并发标记。这是第一个"能演示"的版本。

### 阶段 5 — 配置系统（M3a，小）

- `config.toml`：热键组合、多轮盘（不同修饰键对应不同轮盘，如 Alt=主轮盘、
  Ctrl=副轮盘）、每个轮盘的条目（label、颜色、图标）、动画参数。
- 首次运行生成默认配置；notify 监听文件变更热重载；校验错误可提示。

**验收**：改 TOML 保存后，轮盘内容即时生效，无需重启。

### 阶段 6 — 托盘 + 设置窗口（M3b，中）

- 托盘（tray-icon）：菜单「设置 / 暂停 / 开机自启 / 退出」；暂停时钩子直通。
- 设置窗口（普通 egui 窗口，同一套渲染栈）：轮盘条目增删改排序、颜色、
  热键录制控件、轮盘实时预览；落盘仍是 config.toml（单一数据源）。

**验收**：不碰配置文件也能完成全部定制；暂停/恢复生效。

### 阶段 7 — 打磨与 v0.1.0 发布（M4，中）

- 单实例守卫（命名 Mutex）、开机自启（HKCU Run）、应用图标与版本资源内嵌。
- 性能验收：空闲 CPU ≈ 0%；热键按下→轮盘可见 < 50ms（tracing 打点验证）；
  常驻内存目标 < 50MB。
- 打包：cargo-packager(NSIS) 或 Inno Setup 出安装包；GitHub Actions tag 触发 Release。
- README（含截图/演示 GIF）、已知限制章节（全屏独占、管理员进程）。

**验收**：干净机器安装即用；v0.1.0 tag + Release 附件。

## 暂缓项（明确不进 MVP）

多人房间共享（Nostr/E2EE）、AI 队友、自动更新（手动下 Release 即可）、
macOS 支持（platform trait 已预留）、自定义图片表情、音效、多主题皮肤。

## 模块布局（阶段 0 落地）

```
src/
  lib.rs         — 库入口：pub mod 汇总（bin 经由 lib 引用，pub 项不触发 dead_code）
  main.rs        — 启动、线程编排（tracing 初始化 + 钩子线程 + winit 事件循环）
  app.rs         — 全局状态编排
  gesture.rs     — 手势状态机（纯逻辑，可单测）
  wheel.rs       — 轮盘几何与命中测试（纯逻辑，可单测）
  marker.rs      — 标记实体与动画曲线（纯逻辑，可单测）
  config.rs      — TOML 加载/校验/热重载
  render/        — egui 绘制（轮盘、标记、设置窗口）
  platform/
    mod.rs       — 输入/覆盖层/托盘的平台抽象 trait
    win32/       — Windows 实现（hooks、窗口样式、托盘、注册表）
```

## 风险与已知限制

| 风险 | 影响 | 对策 |
|---|---|---|
| 全屏独占模式下覆盖层不可见 | 游戏内不可用 | 无边框/窗口化可用（lolPing 同限）；README 说明 |
| 低级钩子钩不到管理员进程 | 部分场景失效 | 文档提示；可选提供以管理员运行的清单变体 |
| 杀软对钩子+全局覆盖层误报 | 分发受阻 | 发布时观察，必要时提交白名单申请 |
| Alt 系统行为（菜单激活）干扰 | 手势被打断 | 钩子中吞掉手势期间的键盘/鼠标事件（阶段 2 处理） |
| 多显示器 DPI 坐标错位 | 轮盘/标记偏移 | PerMonitorV2 + winit scale，阶段 1 就用双屏验证 |
