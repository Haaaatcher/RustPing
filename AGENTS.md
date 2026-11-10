# RustPing — 工作区说明

## 项目定位

用 Rust 编写的桌面应用，仿造 [lolPing](https://github.com/darrenprx/lolPing)。

**注意：本项目与网络 ping（延迟监测）无关。** 虽然名字里有 "Ping"，lolPing 实际是"桌面 Ping 轮盘"：按住热键（Alt）拖拽唤出轮盘，松开后通过透明、置顶、可穿透点击的覆盖层窗口在屏幕任意位置打出 ping/表情标记，用于直播、演示、协作时指示屏幕位置。

### 与 lolPing 的差异（本项目目标）

- **更通用**：舍弃与 League of Legends 的绑定。不使用 Riot 的 ping/表情素材（版权归 Riot，不在 lolPing 的 MIT 许可内），轮盘内容完全通用、可自定义。
- **更快速**：只保留关键功能，性能优先，GPU 加速渲染。参考实现 lolPing 用 Electron + React + C++ 原生辅助进程；本项目用 Rust 重写以追求性能。

### 技术选型（2026-11-08 确认，2026-11-10 修订）

- GUI/渲染：**egui + wgpu**，纯 Rust、GPU 加速，覆盖层与设置窗口共用一套栈；不手写着色器。
- 窗口/事件循环：**winit 直接集成**（egui-winit + egui-wgpu 手动接线）。原定 eframe，阶段 1 发现其单主窗口假设与"每显示器一个覆盖窗口"冲突（不暴露显示器枚举、root 窗口冗余、per-viewport 样式控制绕远路），已移除；渲染栈本身不变。
- click-through 等扩展样式：raw-window-handle 拿 HWND 调 Win32，实现于 `platform/win32`（trait `OverlayWindowExt`，ex-style 写后回读校验）。
- 平台策略：**Windows 优先**；平台相关代码（输入钩子/覆盖层/托盘）收敛在 `src/platform/` 并抽 trait，macOS 作为后续独立阶段。
- 开发顺序：见 [docs/ROADMAP.md](docs/ROADMAP.md)（M0 骨架 → M1 平台层 → M2 核心闭环 → M3 可配置 → M4 发布）。
- 开发流程与调试：见 [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)。要点——每阶段节奏为"纯逻辑+单测先行 → 平台 Spike（examples）→ 集成 → 冒烟清单"；调试主通道是日志+HUD（窗口看不见、点不着）；**红线：LL 钩子回调内禁止断点与慢逻辑（超时会被系统静默摘钩），回调只入队事件**；debug 构建有急停热键（钩子线程处理）与心跳看门狗，防止全局吞输入把开发机锁死。

### lolPing 实现要点（参考）

- C++ 原生辅助进程拦截 Alt+拖拽（Windows 低级输入钩子 / macOS event tap），通过 stdin/stdout 的 JSON lines 与主进程通信。
- 每个显示器一个透明、置顶、可穿透点击（click-through）的覆盖层窗口渲染轮盘与 ping。
- 房间共享、AI 队友、自动更新等属于 lolPing 的高级功能，按"只保留关键功能"原则取舍。

## 当前状态（2026-11-10）

- **M0 已完成**（tag v0.0.1）：完整日志系统（`src/logging.rs`）、纯逻辑桩模块 + 8 个单测、CI。
- **阶段 1（M1a 覆盖层窗口）代码完成，程序化验收已过，交互项待用户冒烟**：每显示器透明/置顶/无边框/不抢焦点覆盖窗口（winit+egui-wgpu 直连）、click-through 可切换、按需渲染（空闲 CPU ≈0.5%）、PerMonitorV2 manifest（build.rs + winresource 嵌入）。Spike 在 `examples/spike_overlay_window.rs`（透明窗口）与 `examples/spike_clickthrough.rs`（空格切换穿透）。
- 常用命令：`cargo run`（发布行为：全透明+穿透，无任务栏入口）；`RUSTPING_DEBUG=1 cargo run`（debug 覆盖层：半透明底+描边+标注文字、可点击、Esc 退出）；`cargo test` / `cargo clippy -- -D warnings`；日志在 `%LOCALAPPDATA%\RustPing\logs`。
- 下一阶段：阶段 2（全局输入捕获，见 ROADMAP）。动手前先确认当前实际结构，再更新本文件。
- 开发顺序与阶段明细：[docs/ROADMAP.md](docs/ROADMAP.md)；开发流程与调试：[docs/DEVELOPMENT.md](docs/DEVELOPMENT.md)。
- git 远程仓库 https://github.com/Haaaatcher/RustPing（origin，main 分支），里程碑打 tag（M0 = v0.0.1）。

## 环境与约定

- 开发环境：Windows（win32）+ Git Bash。注意路径分隔符与 shell 兼容性。
- 覆盖层窗口、全局热键、低级输入拦截等均依赖平台 API，跨平台改动需同时考虑 Windows 与 macOS 行为。
