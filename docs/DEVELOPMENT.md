# 开发流程与调试指南

[ROADMAP.md](ROADMAP.md) 定义"做什么、什么顺序"，本文定义"怎么干、怎么调"。

## 1. 每阶段的固定节奏

```
① 接口设计 → ② 纯逻辑+单测 → ③ 平台 Spike → ④ 集成 → ⑤ 冒烟清单 → ⑥ 提交记录
```

- **① 接口设计**：先定 struct/trait 边界（如 `GestureEvent`、`WheelLayout`），平台细节不进纯逻辑模块。
- **② 纯逻辑 + 单测先行**：状态机迁移表、轮盘角度命中、动画曲线先用查表/参数化测试覆盖，`cargo test` 不开窗口即可跑。CI 全绿是进入集成的门槛。
- **③ 平台 Spike**：`examples/` 下最小程序逐个验证 Win32 能力，早失败：
  - `spike_overlay_window.rs` — 透明、置顶、无边框窗口 + 多显示器
  - `spike_clickthrough.rs` — `WS_EX_TRANSPARENT` 动态开关
  - `spike_lowlevel_hooks.rs` — `WH_KEYBOARD_LL`/`WH_MOUSE_LL` 收事件并打印

  Spike 同时充当平台 interop 的可运行文档。
- **④ 集成**：Spike 验证过的代码收编进 `platform/win32/`，业务逻辑零改动。
- **⑤ 冒烟清单**：跑第 8 节对应清单，结果记入收尾 commit message。
- **⑥ 提交**：main 小步直推，里程碑打 tag（单人项目不开 PR 流程；CI 对每次 push 跑 fmt/clippy/test/build）。

## 2. 分层调试策略

| 层 | 内容 | 手段 | 环境 |
|---|---|---|---|
| 纯逻辑 | gesture / wheel / marker / config | `cargo test`、参数化用例 | CI + 本地 |
| 平台层 | hooks / overlay / tray | `cargo run --example spike_*` | 仅本地 Windows |
| 集成层 | 全链路 | 调试模式运行 + tracing 日志 + debug HUD | 仅本地 |
| 性能 | 延迟 / 帧耗时 / CPU | tracing span 打点 + HUD + 系统监视器 | 本地 |

核心原则：**窗口看不见、点不着、不抢焦点，所以调试的主观察通道是日志和 HUD，不是窗口交互。**

## 3. 调试模式（仅 debug 构建生效）

环境变量 `RUSTPING_DEBUG=1` 开启：

- **看得见**：覆盖层加半透明底色 + 描边，关闭 click-through（可点击自己）。发布构建完全不受影响。
- **HUD 面板**：手势状态机当前态、存活标记数、每秒事件数、钩子心跳计数、上一帧耗时。
- **吞输入开关**：`RUSTPING_DEBUG_NO_SWALLOW=1` 时钩子只记录不吞——调试手势时不锁死自己的鼠标。
- **时间缩放**：`RUSTPING_DEBUG_TIME_SCALE=0.2` 把动画慢放 5 倍，逐帧核对曲线。
- egui 自带检查面板（settings / inspection / memory UI）在 debug 模式下启用。

## 4. 保险机制（防"调试把自己锁在电脑外"）

全局吞输入的风险用两道保险兜住：

1. **急停热键**：钩子线程内识别 `Ctrl+Alt+Shift+F12` → 直接退出进程。进程退出后系统自动摘钩，输入立即恢复。注意：该热键**必须由钩子线程处理**（即使钩子正在吞输入，它也收得到），不能依赖 UI 线程存活。
2. **看门狗**：UI 线程每 250ms 心跳；钩子线程发现心跳超时 3s → 主动 `UnhookWindowsHookEx` 停止吞输入并置错误态。

安全性质：钩子随进程消亡自动失效——**崩溃 = 输入恢复**，不会永久吞掉用户输入（前提：吞输入只发生在钩子回调内）。

## 5. 钩子调试红线

- **LL 钩子回调内禁止断点**。回调若在 `LowLevelHooksTimeout`（默认数百毫秒）内不返回，系统会**静默摘除钩子**——表现为"没改代码但钩子突然死了"，无任何通知。所有断点打在 handler 线程。
- 回调只做三件事：读事件 → 入 channel → return。目标 <50µs，杜绝堆分配与锁等待。
- **钩子可能被系统摘除**（回调超时、会话切换/RDP 等），因此：心跳计数进 HUD；心跳停止增长 → 自动重装钩子。可观测性与健壮性用同一套机制。
- 验证窗口样式用 **Spy++**（VS 自带）：确认 `WS_EX_TRANSPARENT | WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` 实际生效，不靠猜。

## 6. 日志与事后分析

- tracing 双层输出：console（pretty）+ 滚动文件（tracing-appender），目录 `%LOCALAPPDATA%\RustPing\logs`。
- 自定义 panic hook：panic 信息 + backtrace 写文件后再退出，崩溃可事后分析。
- 关键 span：`hook_event`（回调入口）、`gesture_transition`、`paint`。

## 7. 性能量度方法（对应 M4 验收）

| 指标 | 度量点 | 工具 |
|---|---|---|
| 热键→轮盘首帧 <50ms | 钩子回调记 `t0` → 单帧绘制完成记 `t1`（近似呈现时刻） | tracing span + HUD 常显 |
| 空闲 CPU ≈ 0% | 静置 5 分钟基线；确认无动画时不 request_repaint | 任务管理器 / Process Explorer |
| 动画帧耗时 | HUD 显示上一帧 paint 耗时，目标 16.6ms 内 | HUD |
| 命中测试微基准 | `wheel::hit_test` 批量执行 | `cargo bench`（可选 criterion） |

注意：`t1` 是"绘制完成"而非"显示器呈现完成"，略有低估，作为验收口径足够。

## 8. 手动冒烟清单（按里程碑）

### M1a 覆盖层
- [ ] debug 模式覆盖层叠在记事本上可见
- [ ] 发布模式点击穿透到下层窗口
- [ ] 任务栏无图标、Alt-Tab 无条目
- [ ] 启动不抢焦点：在编辑器连续打字，覆盖层弹出期间输入不中断
- [ ] 双显示器位置/尺寸正确
- [ ] 100%/125%/150% 缩放、混合 DPI 双屏下几何不错位

### M1b 钩子
- [ ] 任意应用上热键按下/抬起日志可见
- [ ] 手势期间下层应用收不到鼠标事件
- [ ] 手势期间按 Alt 不激活下层应用菜单
- [ ] Esc 取消手势，输入恢复正常
- [ ] 锁屏/解锁后钩子心跳仍在走
- [ ] UAC 安全桌面期间输入吞不掉（预期行为，记录即可）

### M2 轮盘/标记
- [ ] 热键→轮盘可见 <50ms（HUD 读数）
- [ ] hover 高亮与命中测试在 8 个方向各验证一次
- [ ] 未过死区松开 = 取消
- [ ] 连打 10 个标记并发播放不掉帧，动画结束存活数归零
- [ ] 慢放模式逐帧核对出现/淡出曲线

### M3 配置/托盘
- [ ] 改 `config.toml` 保存 → 轮盘即时变化，无需重启
- [ ] 写入非法 TOML → 提示错误且沿用旧配置继续运行
- [ ] 托盘暂停 → 热键失效；恢复 → 正常
- [ ] 设置窗口修改与手改文件冲突时，mtime 新者胜

## 9. CI 边界

- CI 只跑：`cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test`（纯逻辑）、`cargo build`。
- 平台行为（钩子/覆盖层/托盘）**不进 CI**：runner 无交互桌面，`SendInput` + 钩子的注入式测试不可靠，这类验证完全依赖第 8 节冒烟清单。
- 纯逻辑测试必须不依赖显示器与输入设备——这也是"② 纯逻辑先行"带来的强制解耦。
