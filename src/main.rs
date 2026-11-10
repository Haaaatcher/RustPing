//! 启动与线程编排：日志初始化 → winit 事件循环（每显示器一个覆盖窗口）。
//! 阶段 2 起在此增加钩子线程与手势事件 channel 的编排。

use std::sync::Arc;

use egui_wgpu::wgpu::CurrentSurfaceTexture;
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor, wgpu};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    monitor::MonitorHandle,
    window::{Window, WindowAttributes, WindowId, WindowLevel},
};

use rustping::{app::OverlayUi, logging, platform::OverlayWindowExt, render};

fn main() {
    let _log_guard = logging::init();

    // debug 模式：覆盖层可见可点（半透明底+描边），发布模式全透明+穿透
    let is_debug = std::env::var_os("RUSTPING_DEBUG").is_some();
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        is_debug,
        "RustPing 启动"
    );

    let event_loop = EventLoop::new().expect("event loop");
    let mut program = OverlayProgram {
        debug: is_debug,
        gpu: None,
        ctx: egui::Context::default(),
        ui: OverlayUi { debug: is_debug },
        wins: Vec::new(),
    };
    render::install_cjk_fallback(&program.ctx);
    event_loop.run_app(&mut program).expect("run");
}

struct GpuShared {
    /// 持有 instance 以保活所有 surface（surface 依赖 instance 存活）
    #[expect(dead_code, reason = "仅保活用途；热插拔重适配在阶段 2+ 再用")]
    instance: wgpu::Instance,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

struct OverlayWindowState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    egui_state: egui_winit::State,
}

struct OverlayProgram {
    debug: bool,
    gpu: Option<GpuShared>,
    ctx: egui::Context,
    ui: OverlayUi,
    wins: Vec<OverlayWindowState>,
}

impl OverlayProgram {
    fn create_windows(&mut self, event_loop: &ActiveEventLoop) {
        let monitors: Vec<MonitorHandle> = event_loop.available_monitors().collect();
        if monitors.is_empty() {
            tracing::error!("没有可用显示器，退出");
            event_loop.exit();
            return;
        }
        tracing::info!(count = monitors.len(), "枚举到显示器");

        use winit::platform::windows::WindowAttributesExtWindows as _;
        let mut windows = Vec::with_capacity(monitors.len());
        for (i, m) in monitors.iter().enumerate() {
            let attrs: WindowAttributes = Window::default_attributes()
                .with_title(format!("RustPing overlay {i}"))
                .with_decorations(false)
                .with_transparent(true)
                .with_window_level(WindowLevel::AlwaysOnTop)
                .with_active(false)
                .with_resizable(false)
                .with_skip_taskbar(true)
                .with_position(m.position())
                .with_inner_size(m.size());
            let window = Arc::new(
                event_loop
                    .create_window(attrs)
                    .unwrap_or_else(|e| panic!("创建覆盖窗口 {i} 失败: {e}")),
            );
            let applied = window.apply_overlay_style();
            let click_through = !self.debug;
            let click_through_ok = window.set_click_through(click_through);
            tracing::info!(
                i,
                pos = ?m.position(),
                size = ?m.size(),
                scale = m.scale_factor(),
                applied,
                click_through,
                click_through_ok,
                ex = window.ex_style().map(|v| format!("{v:#x}")),
                "覆盖窗口已创建并设置样式"
            );
            windows.push(window);
        }

        // GPU 初始化：instance/adapter/device 全窗口共享（eframe 同款策略）
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let first_surface = instance
            .create_surface(windows[0].clone())
            .expect("create surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&first_surface),
            force_fallback_adapter: false,
            ..Default::default()
        }))
        .expect("request adapter");
        tracing::info!(info = ?adapter.get_info(), "wgpu adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("rustping"),
            ..Default::default()
        }))
        .expect("request device");

        for window in windows {
            let surface = instance
                .create_surface(window.clone())
                .expect("create surface");
            let caps = surface.get_capabilities(&adapter);
            // 透明合成：非 sRGB + PreMultiplied 优先
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| !f.is_srgb())
                .unwrap_or(caps.formats[0]);
            let alpha_mode = caps
                .alpha_modes
                .iter()
                .copied()
                .find(|a| matches!(a, wgpu::CompositeAlphaMode::PreMultiplied))
                .unwrap_or(caps.alpha_modes[0]);
            let size = window.inner_size();
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
                color_space: wgpu::SurfaceColorSpace::Auto,
            };
            surface.configure(&device, &config);
            let egui_state = egui_winit::State::new(
                self.ctx.clone(),
                egui::ViewportId(egui::Id::unique(window.id())),
                &window,
                Some(window.scale_factor() as f32),
                None,
                None,
            );
            let renderer = Renderer::new(
                &device,
                format,
                RendererOptions {
                    msaa_samples: 1,
                    ..Default::default()
                },
            );
            self.wins.push(OverlayWindowState {
                window,
                surface,
                config,
                renderer,
                egui_state,
            });
        }

        self.gpu = Some(GpuShared {
            instance,
            device,
            queue,
        });

        // 初始绘制一帧；此后按需（事件驱动）重绘
        for w in &self.wins {
            w.window.request_redraw();
        }
    }

    fn render_window(&mut self, idx: usize) {
        let count = self.wins.len();
        let Some(gpu) = &self.gpu else { return };
        let Some(w) = self.wins.get_mut(idx) else {
            return;
        };

        let raw_input = w.egui_state.take_egui_input(&w.window);
        // 先取帧再跑 egui：取帧失败提前返回时不会有未应用的 TexturesDelta 被 drop
        let frame = match w.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(t) | CurrentSurfaceTexture::Suboptimal(t) => t,
            CurrentSurfaceTexture::Outdated => {
                w.surface.configure(&gpu.device, &w.config);
                return;
            }
            // 超时/被遮挡/丢失等：跳过本帧，事件驱动时再试
            _ => return,
        };
        let mut full_output = self.ctx.run_ui(raw_input, |ui| {
            self.ui.draw(ui, idx, count);
        });
        w.egui_state
            .handle_platform_output(&w.window, full_output.platform_output);

        let prims = self
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, deltas) in &full_output.textures_delta.set {
            for delta in deltas {
                w.renderer
                    .update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }

        let size = w.window.inner_size();
        let screen = ScreenDescriptor {
            size_in_pixels: [size.width.max(1), size.height.max(1)],
            pixels_per_point: full_output.pixels_per_point,
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let user_cmds =
            w.renderer
                .update_buffers(&gpu.device, &gpu.queue, &mut encoder, &prims, &screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("overlay pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            w.renderer.render(&mut pass, &prims, &screen);
        }
        for id in &full_output.textures_delta.free {
            w.renderer.free_texture(id);
        }
        // 已全部应用到渲染器；egui 0.37 的 TexturesDelta 在 debug 构建下
        // 检查"未处理即 drop"，处理后需显式清空
        full_output.textures_delta.clear();
        let encoder = encoder.finish();
        gpu.queue
            .submit(user_cmds.into_iter().chain(std::iter::once(encoder)));
        gpu.queue.present(frame);
    }
}

impl ApplicationHandler for OverlayProgram {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.wins.is_empty() {
            self.create_windows(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(idx) = self.wins.iter().position(|w| w.window.id() == window_id) else {
            return;
        };

        // egui_winit 先处理所有事件（拿 repaint 意图）。
        // 注意剔除 RedrawRequested：egui_winit 对它也返回 repaint=true，
        // 若转成 request_redraw 会形成"渲染→再请求"自激循环，满帧率空转 CPU。
        let mut repaint = false;
        if !matches!(event, WindowEvent::RedrawRequested) {
            let w = &mut self.wins[idx];
            repaint = w.egui_state.on_window_event(&w.window, &event).repaint;
        }

        match event {
            WindowEvent::CloseRequested => {
                tracing::info!("CloseRequested → 退出");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => self.render_window(idx),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &self.gpu {
                    let w = &mut self.wins[idx];
                    w.config.width = size.width.max(1);
                    w.config.height = size.height.max(1);
                    w.surface.configure(&gpu.device, &w.config);
                    repaint = true;
                }
            }
            // debug 模式 Esc 退出（覆盖层不进任务栏，得留个退出口）
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } if self.debug => {
                tracing::info!("Esc → 退出");
                event_loop.exit();
            }
            _ => {}
        }

        if repaint {
            self.wins[idx].window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // 阶段 1 按需渲染：无动画无事件时不主动重绘（空闲 CPU ≈ 0）
        // 阶段 4 起在此驱动标记动画的重绘请求
    }
}
