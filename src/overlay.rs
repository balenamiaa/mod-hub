use crate::SHUTDOWN;
use crate::errors::{Error, Result};
use crate::winapi;
pub use egui;
use winit::window::Fullscreen;

use core::sync::atomic::Ordering;

/// Describes a type that renders egui content each frame.
pub trait AppUi: Send + 'static {
    fn ui(&mut self, ctx: &egui::Context);
}

/// Builder for configuring and running a topmost egui overlay window.
#[derive(Clone, Debug)]
pub struct OverlayBuilder {
    title: String,
    inner_size: Option<egui::Vec2>,
    always_on_top: bool,
    transparent: bool,
    decorated: bool,
    resizable: bool,
    fullscreen: bool,
    hide_from_alt_tab: bool,
    show_indicator: bool,
    toggle_vk: i32,
}

impl Default for OverlayBuilder {
    fn default() -> Self {
        Self {
            title: String::from("Overlay"),
            inner_size: None,
            always_on_top: true,
            transparent: true,
            decorated: false,
            resizable: false,
            fullscreen: true,
            hide_from_alt_tab: true,
            show_indicator: true,
            toggle_vk: windows_sys::Win32::UI::Input::KeyboardAndMouse::VK_INSERT as i32,
        }
    }
}

impl OverlayBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn inner_size(mut self, size: egui::Vec2) -> Self {
        self.inner_size = Some(size);
        self
    }

    pub fn always_on_top(mut self, enabled: bool) -> Self {
        self.always_on_top = enabled;
        self
    }

    pub fn transparent(mut self, enabled: bool) -> Self {
        self.transparent = enabled;
        self
    }

    pub fn decorated(mut self, enabled: bool) -> Self {
        self.decorated = enabled;
        self
    }

    pub fn resizable(mut self, enabled: bool) -> Self {
        self.resizable = enabled;
        self
    }

    pub fn fullscreen(mut self, enabled: bool) -> Self {
        self.fullscreen = enabled;
        self
    }

    pub fn hide_from_alt_tab(mut self, enabled: bool) -> Self {
        self.hide_from_alt_tab = enabled;
        self
    }

    pub fn show_indicator(mut self, enabled: bool) -> Self {
        self.show_indicator = enabled;
        self
    }

    pub fn toggle_key(mut self, vk: i32) -> Self {
        self.toggle_vk = vk;
        self
    }

    /// Runs the overlay until the window is closed.
    pub fn run<T>(self, app: T) -> Result<()>
    where
        T: AppUi,
    {
        use egui_wgpu::wgpu;
        use egui_winit::winit;

        struct EguiApp<T: AppUi> {
            ui: T,
            title: String,
            always_on_top: bool,
            transparent: bool,
            decorated: bool,
            resizable: bool,
            fullscreen: bool,
            inner_size: Option<egui::Vec2>,

            window: Option<winit::window::Window>,
            egui_ctx: egui::Context,
            egui_state: Option<egui_winit::State>,

            instance: Option<wgpu::Instance>,
            surface: Option<wgpu::Surface<'static>>,
            device: Option<wgpu::Device>,
            queue: Option<wgpu::Queue>,
            surface_config: Option<wgpu::SurfaceConfiguration>,
            surface_format: Option<wgpu::TextureFormat>,
            renderer: Option<egui_wgpu::Renderer>,

            // Input pass-through toggle (Insert key)
            click_through: bool,
            prev_insert_down: bool,

            hide_from_alt_tab: bool,
            show_indicator: bool,
            toggle_vk: i32,

            // Frame pacing
            last_frame_end: std::time::Instant,
            target_frame: std::time::Duration,
        }

        impl<T: AppUi> EguiApp<T> {
            fn window_mut(&mut self) -> &winit::window::Window {
                self.window.as_ref().unwrap()
            }
            fn device(&self) -> &wgpu::Device {
                self.device.as_ref().unwrap()
            }
            fn queue(&self) -> &wgpu::Queue {
                self.queue.as_ref().unwrap()
            }
            fn surface(&self) -> &wgpu::Surface<'static> {
                self.surface.as_ref().unwrap()
            }
            fn config(&self) -> &wgpu::SurfaceConfiguration {
                self.surface_config.as_ref().unwrap()
            }
            fn config_mut(&mut self) -> &mut wgpu::SurfaceConfiguration {
                self.surface_config.as_mut().unwrap()
            }
            fn renderer_mut(&mut self) -> &mut egui_wgpu::Renderer {
                self.renderer.as_mut().unwrap()
            }

            #[cfg(target_os = "windows")]
            fn apply_click_through(&self, enabled: bool) {
                use raw_window_handle::HasWindowHandle;
                use windows_sys::Win32::Foundation::HWND;
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_TRANSPARENT,
                };

                let hwnd = match self.window.as_ref().and_then(|w| w.window_handle().ok()) {
                    Some(handle) => match handle.as_raw() {
                        raw_window_handle::RawWindowHandle::Win32(h) => h.hwnd.get() as HWND,
                        _ => return,
                    },
                    None => return,
                };
                unsafe {
                    let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                    if enabled {
                        ex |= WS_EX_TRANSPARENT as isize;
                    } else {
                        ex &= !(WS_EX_TRANSPARENT as isize);
                    }
                    let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex);
                }

                // Also hint Winit/Windows to skip hit-testing if available
                #[cfg(target_os = "windows")]
                if let Some(win) = &self.window {
                    let _ = win.set_cursor_hittest(!enabled);
                }

                // Ensure style change takes effect
                #[cfg(target_os = "windows")]
                unsafe {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
                    };
                    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED;
                    let _ = SetWindowPos(hwnd, std::ptr::null_mut(), 0, 0, 0, 0, flags);
                }
            }

            #[cfg(not(target_os = "windows"))]
            fn apply_click_through(&self, _enabled: bool) {}

            fn poll_insert_toggle(&mut self) {
                let down = winapi::is_vk_pressed(self.toggle_vk);
                if down && !self.prev_insert_down {
                    self.click_through = !self.click_through;
                    self.apply_click_through(self.click_through);
                }
                self.prev_insert_down = down;
            }

            #[cfg(target_os = "windows")]
            fn apply_alt_tab_visibility(&self) {
                if !self.hide_from_alt_tab {
                    return;
                }
                use raw_window_handle::HasWindowHandle;
                use windows_sys::Win32::Foundation::HWND;
                use windows_sys::Win32::UI::WindowsAndMessaging::{
                    GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_APPWINDOW,
                    WS_EX_TOOLWINDOW,
                };
                let hwnd = match self.window.as_ref().and_then(|w| w.window_handle().ok()) {
                    Some(handle) => match handle.as_raw() {
                        raw_window_handle::RawWindowHandle::Win32(h) => h.hwnd.get() as HWND,
                        _ => return,
                    },
                    None => return,
                };
                unsafe {
                    let mut ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                    ex &= !(WS_EX_APPWINDOW as isize);
                    ex |= WS_EX_TOOLWINDOW as isize;
                    let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex);
                }
            }
            #[cfg(not(target_os = "windows"))]
            fn apply_alt_tab_visibility(&self) {}
        }

        impl<T: AppUi> winit::application::ApplicationHandler for EguiApp<T> {
            fn resumed(&mut self, elwt: &winit::event_loop::ActiveEventLoop) {
                use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
                use winit::window::{Window, WindowLevel};

                let mut attrs = Window::default_attributes()
                    .with_title(self.title.clone())
                    .with_decorations(self.decorated)
                    .with_transparent(self.transparent)
                    .with_resizable(self.resizable);
                if let Some(size) = self.inner_size {
                    attrs = attrs.with_inner_size(LogicalSize::new(size.x as f64, size.y as f64));
                }
                attrs = attrs.with_window_level(if self.always_on_top {
                    WindowLevel::AlwaysOnTop
                } else {
                    WindowLevel::Normal
                });

                let window = elwt.create_window(attrs).expect("failed to create window");
                self.window = Some(window);

                // Cover screen without true fullscreen (keeps DWM composition for transparency)
                if self.fullscreen {
                    if let Some(w) = &self.window {
                        if let Some(m) = w.current_monitor().or_else(|| elwt.primary_monitor()) {
                            let size = m.size();
                            w.set_outer_position(PhysicalPosition::new(0, 0));
                            let _ =
                                w.request_inner_size(PhysicalSize::new(size.width, size.height));
                        }
                    }
                }

                // Default to click-through
                self.apply_click_through(true);
                // Hide from Alt+Tab / Taskbar if requested
                self.apply_alt_tab_visibility();

                // Make borderless fullscreen to cover the entire screen (configurable)
                if self.fullscreen {
                    if let Some(w) = &self.window {
                        w.set_fullscreen(Some(Fullscreen::Borderless(None)));
                    }
                }

                // WGPU setup
                let instance = wgpu::Instance::default();
                let window_ref = self.window.as_ref().unwrap();
                let surface = unsafe {
                    instance.create_surface_unsafe(
                        wgpu::SurfaceTargetUnsafe::from_window(window_ref).unwrap(),
                    )
                }
                .expect("failed to create surface");
                let adapter =
                    pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::HighPerformance,
                        compatible_surface: Some(&surface),
                        force_fallback_adapter: false,
                    }))
                    .expect("no suitable GPU adapter found");
                let (device, queue) =
                    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                        label: Some("egui-wgpu-device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::default(),
                        memory_hints: Default::default(),
                        trace: Default::default(),
                    }))
                    .expect("request_device failed");

                let caps = surface.get_capabilities(&adapter);
                let surface_format = caps
                    .formats
                    .iter()
                    .copied()
                    .find(|f| f.is_srgb())
                    .unwrap_or(caps.formats[0]);
                let alpha_mode = caps
                    .alpha_modes
                    .iter()
                    .copied()
                    .find(|m| *m == wgpu::CompositeAlphaMode::PreMultiplied)
                    .unwrap_or(caps.alpha_modes[0]);

                let size = window_ref.inner_size();
                let config = wgpu::SurfaceConfiguration {
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: surface_format,
                    width: size.width.max(1),
                    height: size.height.max(1),
                    present_mode: wgpu::PresentMode::Fifo,
                    alpha_mode,
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2,
                };
                surface.configure(&device, &config);

                // egui setup
                let egui_ctx = egui::Context::default();
                let egui_state = egui_winit::State::new(
                    egui_ctx.clone(),
                    egui::ViewportId::ROOT,
                    window_ref,
                    None,
                    None,
                    None,
                );
                let renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1, false);

                self.instance = Some(instance);
                self.surface = Some(surface);
                self.device = Some(device);
                self.queue = Some(queue);
                self.surface_config = Some(config);
                self.surface_format = Some(surface_format);
                self.egui_ctx = egui_ctx;
                self.egui_state = Some(egui_state);
                self.renderer = Some(renderer);

                // Determine refresh rate and target frame interval
                let refresh_hz: u32 = self
                    .window
                    .as_ref()
                    .and_then(|w| w.current_monitor().or_else(|| elwt.primary_monitor()))
                    .and_then(|m| {
                        m.video_modes()
                            .nth(0)
                            .map(|vm| (vm.refresh_rate_millihertz() + 500) / 1000)
                    })
                    .unwrap_or(120);
                let fps = refresh_hz.saturating_add(50).min(1000);
                self.target_frame = std::time::Duration::from_nanos(1_000_000_000u64 / fps as u64);
                self.last_frame_end = std::time::Instant::now();
            }

            fn window_event(
                &mut self,
                _elwt: &winit::event_loop::ActiveEventLoop,
                _id: winit::window::WindowId,
                event: winit::event::WindowEvent,
            ) {
                use winit::event::WindowEvent;
                if let (Some(window), Some(state)) =
                    (self.window.as_ref(), self.egui_state.as_mut())
                {
                    let _ = state.on_window_event(window, &event);
                }

                match event {
                    WindowEvent::CloseRequested => {
                        // Ignore direct close requests; overlay is controlled by SHUTDOWN
                        // to avoid accidental termination when interacting.
                    }
                    WindowEvent::Resized(new_size) => {
                        let surface_ref = self.surface.as_ref().unwrap();
                        let device_ref = self.device.as_ref().unwrap();
                        let mut cfg = self.surface_config.take().unwrap();
                        cfg.width = new_size.width.max(1);
                        cfg.height = new_size.height.max(1);
                        surface_ref.configure(device_ref, &cfg);
                        self.surface_config = Some(cfg);
                        if let Some(w) = &self.window {
                            w.request_redraw();
                        }
                    }
                    WindowEvent::ScaleFactorChanged { .. } => {
                        if let Some(w) = &self.window {
                            let surface_ref = self.surface.as_ref().unwrap();
                            let device_ref = self.device.as_ref().unwrap();
                            let size = w.inner_size();
                            let mut cfg = self.surface_config.take().unwrap();
                            cfg.width = size.width.max(1);
                            cfg.height = size.height.max(1);
                            surface_ref.configure(device_ref, &cfg);
                            self.surface_config = Some(cfg);
                            w.request_redraw();
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        let window_ref = self.window.as_ref().unwrap();
                        let raw_input = {
                            let state = self.egui_state.as_mut().unwrap();
                            state.take_egui_input(window_ref)
                        };

                        let full_output = self.egui_ctx.run(raw_input, |ctx| {
                            self.ui.ui(ctx);
                            if self.show_indicator {
                                use egui::{Align2, Area, RichText};
                                Area::new(egui::Id::new("overlay-indicator"))
                                    .anchor(Align2::LEFT_TOP, egui::vec2(8.0, 8.0))
                                    .interactable(false)
                                    .show(ctx, |ui| {
                                        let text = if self.click_through {
                                            "Pass-through: ON (Ins)"
                                        } else {
                                            "Pass-through: OFF (Ins)"
                                        };
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(0.0, 0.0),
                                            egui::Layout::left_to_right(egui::Align::Min),
                                            |ui| {
                                                let color = if self.click_through {
                                                    egui::Color32::LIGHT_GREEN
                                                } else {
                                                    egui::Color32::YELLOW
                                                };
                                                ui.add(egui::Label::new(
                                                    RichText::new(text).color(color),
                                                ));
                                            },
                                        );
                                    });
                            }
                        });

                        let device = self.device.as_ref().unwrap();
                        let queue = self.queue.as_ref().unwrap();
                        {
                            let renderer = self.renderer.as_mut().unwrap();
                            for (id, delta) in &full_output.textures_delta.set {
                                renderer.update_texture(device, queue, *id, delta);
                            }
                            for id in &full_output.textures_delta.free {
                                renderer.free_texture(id);
                            }
                        }

                        let clipped = self
                            .egui_ctx
                            .tessellate(full_output.shapes, full_output.pixels_per_point);

                        let sz = window_ref.inner_size();
                        let ppp = egui_winit::pixels_per_point(&self.egui_ctx, window_ref);
                        let screen = egui_wgpu::ScreenDescriptor {
                            size_in_pixels: [sz.width, sz.height],
                            pixels_per_point: ppp,
                        };

                        let mut encoder =
                            self.device()
                                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                    label: Some("egui-wgpu-encoder"),
                                });

                        {
                            let renderer = self.renderer.as_mut().unwrap();
                            renderer.update_buffers(device, queue, &mut encoder, &clipped, &screen);
                        }

                        let surface_ref = self.surface.as_ref().unwrap();
                        let surface_texture = match surface_ref.get_current_texture() {
                            Ok(frame) => frame,
                            Err(_) => {
                                let device_ref = self.device.as_ref().unwrap();
                                let cfg = self.surface_config.take().unwrap();
                                surface_ref.configure(device_ref, &cfg);
                                self.surface_config = Some(cfg);
                                match surface_ref.get_current_texture() {
                                    Ok(frame) => frame,
                                    Err(_) => return,
                                }
                            }
                        };
                        let view = surface_texture
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        // Begin render pass in its own scope so it is dropped
                        // before we finish the command encoder.
                        {
                            let renderer = self.renderer.as_mut().unwrap();
                            let render_pass_descriptor = wgpu::RenderPassDescriptor {
                                label: Some("egui-wgpu-rpass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            };
                            let render_pass = encoder.begin_render_pass(&render_pass_descriptor);
                            let mut static_render_pass = render_pass.forget_lifetime();
                            renderer.render(&mut static_render_pass, &clipped, &screen);
                            // Dropped at end of this scope to unlock encoder
                        }

                        queue.submit(std::iter::once(encoder.finish()));
                        surface_texture.present();

                        // Mark frame end for pacing
                        self.last_frame_end = std::time::Instant::now();

                        let window_ref = self.window.as_ref().unwrap();
                        if let Some(state) = self.egui_state.as_mut() {
                            state.handle_platform_output(window_ref, full_output.platform_output);
                        }
                    }
                    _ => {}
                }
            }

            fn about_to_wait(&mut self, elwt: &winit::event_loop::ActiveEventLoop) {
                if SHUTDOWN.load(Ordering::SeqCst) {
                    elwt.exit();
                    return;
                }

                // Poll Insert and apply click-through toggle first to avoid borrow conflict
                self.poll_insert_toggle();

                // Frame pacing: sleep for remaining time to target
                let now = std::time::Instant::now();
                if now < self.last_frame_end + self.target_frame {
                    let remain = (self.last_frame_end + self.target_frame) - now;
                    // Use small sleeps to improve accuracy on Windows
                    if remain >= std::time::Duration::from_micros(200) {
                        std::thread::sleep(remain - std::time::Duration::from_micros(200));
                    }
                    // Busy-wait the final ~200µs for better precision
                    while std::time::Instant::now() < self.last_frame_end + self.target_frame {}
                }

                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
        }

        let event_loop = {
            let mut builder = winit::event_loop::EventLoop::builder();
            #[cfg(target_os = "windows")]
            {
                use winit::platform::windows::EventLoopBuilderExtWindows;
                builder.with_any_thread(true);
            }
            builder.build().map_err(|e| Error::Run(e.to_string()))?
        };

        let mut app = EguiApp::<T> {
            ui: app,
            title: self.title,
            always_on_top: self.always_on_top,
            transparent: self.transparent,
            decorated: self.decorated,
            resizable: self.resizable,
            fullscreen: self.fullscreen,
            inner_size: self.inner_size,
            window: None,
            egui_ctx: egui::Context::default(),
            egui_state: None,
            instance: None,
            surface: None,
            device: None,
            queue: None,
            surface_config: None,
            surface_format: None,
            renderer: None,
            click_through: true,
            prev_insert_down: false,
            hide_from_alt_tab: self.hide_from_alt_tab,
            show_indicator: self.show_indicator,
            toggle_vk: self.toggle_vk,
            last_frame_end: std::time::Instant::now(),
            target_frame: std::time::Duration::from_millis(0),
        };

        event_loop
            .run_app(&mut app)
            .map_err(|e| Error::Run(format!("event loop error: {e}")))
    }
}

/// Runs an overlay window with defaults.
pub fn run_overlay<T: AppUi>(app: T) -> Result<()> {
    OverlayBuilder::new().run(app)
}

// Controlled by the host crate.
// SHUTDOWN is provided by the crate root (lib.rs)
