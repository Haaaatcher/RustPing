# RustPing — 工作区说明

## 项目定位

用 Rust 编写的桌面应用，仿造 [lolPing](https://github.com/darrenprx/lolPing)。

**注意：本项目与网络 ping（延迟监测）无关。** 虽然名字里有 "Ping"，lolPing 实际是"桌面 Ping 轮盘"：按住热键（Alt）拖拽唤出轮盘，松开后通过透明、置顶、可穿透点击的覆盖层窗口在屏幕任意位置打出 ping/表情标记，用于直播、演示、协作时指示屏幕位置。

### 与 lolPing 的差异（本项目目标）

- **更通用**：舍弃与 League of Legends 的绑定。不使用 Riot 的 ping/表情素材（版权归 Riot，不在 lolPing 的 MIT 许可内），轮盘内容完全通用、可自定义。
- **更快速**：只保留关键功能，性能优先，GPU 加速渲染。参考实现 lolPing 用 Electron + React + C++ 原生辅助进程；本项目用 Rust 重写以追求性能。

### lolPing 实现要点（参考）

- C++ 原生辅助进程拦截 Alt+拖拽（Windows 低级输入钩子 / macOS event tap），通过 stdin/stdout 的 JSON lines 与主进程通信。
- 每个显示器一个透明、置顶、可穿透点击（click-through）的覆盖层窗口渲染轮盘与 ping。
- 房间共享、AI 队友、自动更新等属于 lolPing 的高级功能，按"只保留关键功能"原则取舍。

## 当前状态（2026-11-07）

- 工作区为空，项目尚未初始化（无 Cargo.toml）。GUI/渲染技术栈（winit + wgpu/egui、tauri 等）尚未选型——修改前先确认当前实际结构，再更新本文件。
- 尚未初始化 git。

## 环境与约定

- 开发环境：Windows（win32）+ Git Bash。注意路径分隔符与 shell 兼容性。
- 覆盖层窗口、全局热键、低级输入拦截等均依赖平台 API，跨平台改动需同时考虑 Windows 与 macOS 行为。
