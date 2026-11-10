//! Spike 2：WS_EX_TRANSPARENT click-through 动态切换。
//!
//! 验证：经 raw-window-handle 拿 HWND → SetWindowLongPtrW(GWL_EXSTYLE) 的
//! 穿透开关真实生效（写入后回读校验）。空格切换，Esc 退出。
//! 使用 `rustping::platform` 的正式实现（spike 提前验证它）。

use std::sync::Arc;

use egui_wgpu::wgpu::{
    self, CompositeAlphaMode, CurrentSurfaceTexture, RenderPassDescriptor, SurfaceColorSpace,
    SurfaceConfiguration,
};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
use rustping::platform::OverlayWindowExt;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowLevel},
};

fn main() {
    rustping::logging::init();
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = SpikeApp {
        click_through: false,
        ..Default::default()
    };
    event_loop.run_app(&mut app).expect("run");
}

#[derive(Default)]
struct SpikeApp {
    win: Option<WindowState>,
    click_through: bool,
}

struct WindowState {
    ctx: egui::Context,
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    config: SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    egui_winit: egui_winit::State,
}

impl SpikeApp {
    fn create_window(&mut self, event_loop: &ActiveEventLoop) {
        let Some(monitor) = event_loop.available_monitors().next() else {
            panic!("没有可用显示器");
        };
        use winit::platform::windows::WindowAttributesExtWindows as _;
        let attrs: WindowAttributes = Window::default_attributes()
            .with_title("RustPing spike: clickthrough")
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_active(false)
            .with_resizable(false)
            .with_skip_taskbar(true)
            .with_position(monitor.position())
            .with_inner_size(monitor.size());
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));

        let applied = window.apply_overlay_style();
        let ok = window.set_click_through(self.click_through);
        tracing::info!(
            applied,
            click_through_set = ok,
            ex = window.ex_style().map(|v| format!("{v:#x}")),
            "初始样式（穿透=关）"
        );

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window.clone())
            .expect("create surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            ..Default::default()
        }))
        .expect("request adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("rustping-spike-ct"),
            ..Default::default()
        }))
        .expect("request device");

        let caps = surface.get_capabilities(&adapter);
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
            .find(|a| matches!(a, CompositeAlphaMode::PreMultiplied))
            .unwrap_or(caps.alpha_modes[0]);
        let size = window.inner_size();
        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: SurfaceColorSpace::Auto,
        };
        surface.configure(&device, &config);

        let egui_ctx = egui::Context::default();
        rustping::render::install_cjk_fallback(&egui_ctx);
        let egui_winit = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
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
        self.win = Some(WindowState {
            ctx: egui_ctx,
            window,
            surface,
            config,
            device,
            queue,
            renderer,
            egui_winit,
        });
    }

    fn render(&mut self) {
        let Some(s) = &mut self.win else { return };
        let ct = self.click_through;

        let raw_input = s.egui_winit.take_egui_input(&s.window);
        let mut full_output = s.ctx.run_ui(raw_input, move |ui| {
            let rect = ui.max_rect();
            let c = rect.center();
            let color = if ct {
                egui::Color32::from_rgb(230, 80, 60)
            } else {
                egui::Color32::from_rgb(0, 210, 130)
            };
            ui.painter()
                .circle_stroke(c, 80.0, egui::Stroke::new(10.0, color));
            ui.painter().circle_filled(c, 6.0, color);
            ui.painter().text(
                c + egui::vec2(0.0, 105.0),
                egui::Align2::CENTER_TOP,
                format!(
                    "穿透: {}（空格切换，Esc 退出）",
                    if ct { "开" } else { "关" }
                ),
                egui::FontId::proportional(20.0),
                egui::Color32::WHITE,
            );
        });
        s.egui_winit
            .handle_platform_output(&s.window, full_output.platform_output);

        let prims = s
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, deltas) in &full_output.textures_delta.set {
            for delta in deltas {
                s.renderer.update_texture(&s.device, &s.queue, *id, delta);
            }
        }

        let frame = match s.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(t) | CurrentSurfaceTexture::Suboptimal(t) => t,
            CurrentSurfaceTexture::Outdated => {
                s.surface.configure(&s.device, &s.config);
                return;
            }
            _ => return,
        };
        let size = s.window.inner_size();
        let screen = ScreenDescriptor {
            size_in_pixels: [size.width.max(1), size.height.max(1)],
            pixels_per_point: full_output.pixels_per_point,
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = s
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let user_cmds =
            s.renderer
                .update_buffers(&s.device, &s.queue, &mut encoder, &prims, &screen);
        {
            let mut pass = encoder
                .begin_render_pass(&RenderPassDescriptor {
                    label: Some("egui pass"),
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
            s.renderer.render(&mut pass, &prims, &screen);
        }
        for id in &full_output.textures_delta.free {
            s.renderer.free_texture(id);
        }
        full_output.textures_delta.clear();
        let encoder = encoder.finish();
        s.queue
            .submit(user_cmds.into_iter().chain(std::iter::once(encoder)));
        s.queue.present(frame);
    }

    fn toggle(&mut self) {
        let Some(s) = &self.win else { return };
        self.click_through = !self.click_through;
        let ok = s.window.set_click_through(self.click_through);
        tracing::info!(
            click_through = self.click_through,
            ok,
            ex = s.window.ex_style().map(|v| format!("{v:#x}")),
            "切换穿透"
        );
        s.window.request_redraw();
    }
}

impl ApplicationHandler for SpikeApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.create_window(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if let Some(s) = &mut self.win {
            let resp = s.egui_winit.on_window_event(&s.window, &event);
            if resp.repaint {
                s.window.request_redraw();
            }
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.render(),
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Space),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => self.toggle(),
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => event_loop.exit(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(s) = &self.win {
            s.window.request_redraw();
        }
    }
}
