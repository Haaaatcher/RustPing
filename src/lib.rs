//! RustPing — 通用桌面 Ping 轮盘（仿 lolPing，去 LoL 绑定）。
//!
//! 模块划分见 `docs/ROADMAP.md`；开发节奏与调试手段见 `docs/DEVELOPMENT.md`。
//! lib + bin 拆分：业务代码都在库里（pub 项不触发 dead_code），
//! `main.rs` 只做启动与线程编排。

pub mod app;
pub mod config;
pub mod gesture;
pub mod logging;
pub mod marker;
pub mod platform;
pub mod render;
pub mod wheel;
