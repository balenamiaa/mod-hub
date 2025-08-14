use crate::errors::{Error, Result};
use crate::winapi;
use std::mem::MaybeUninit;
use std::ptr::null_mut;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::System::Com::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

fn get_x_lparam(lp: LPARAM) -> i16 {
    (lp.0 & 0xFFFF) as i16
}

fn get_y_lparam(lp: LPARAM) -> i16 {
    ((lp.0 >> 16) & 0xFFFF) as i16
}

mod d3d;
mod dcomp;
mod painter_d3d;
mod util;
mod win;

pub trait AppUi: Send + 'static {
    fn ui(&mut self, ctx: &egui::Context);
}

#[derive(Clone, Debug)]
pub struct OverlayBuilder {
    pub title: String,
    pub hide_from_alt_tab: bool,
    pub width: i32,
    pub height: i32,
    pub click_through_on_start: bool,
    pub toggle_vk: i32,
}

impl Default for OverlayBuilder {
    fn default() -> Self {
        Self {
            title: String::from("Overlay"),
            hide_from_alt_tab: true,
            width: unsafe { GetSystemMetrics(SM_CXSCREEN) },
            height: unsafe { GetSystemMetrics(SM_CYSCREEN) },
            click_through_on_start: true,
            toggle_vk: VK_INSERT.0 as i32,
        }
    }
}

impl OverlayBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = t.into();
        self
    }
    pub fn size(mut self, w: i32, h: i32) -> Self {
        self.width = w;
        self.height = h;
        self
    }
    pub fn hide_from_alt_tab(mut self, v: bool) -> Self {
        self.hide_from_alt_tab = v;
        self
    }
    pub fn click_through(mut self, v: bool) -> Self {
        self.click_through_on_start = v;
        self
    }
    pub fn toggle_key(mut self, vk: i32) -> Self {
        self.toggle_vk = vk;
        self
    }

    pub fn run<T: AppUi>(self, mut app: T) -> Result<()> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|e| Error::Run(format!("CoInitializeEx: {e}")))?;
        }
        winapi::debug_log("overlay: COM initialized");

        let class =
            win::register_window_class("restident_overlay_wnd").map_err(|e| Error::Create(e))?;
        let owner_class =
            win::register_window_class("restident_overlay_owner").map_err(|e| Error::Create(e))?;
        winapi::debug_log("overlay: window classes registered");

        let owner =
            win::create_owner_window(owner_class, &self.title).map_err(|e| Error::Create(e))?;
        winapi::debug_log(&format!("overlay: owner hwnd=0x{:x}", owner.0 as usize));

        let hwnd = win::create_overlay_window(
            class,
            owner,
            &self.title,
            self.width,
            self.height,
            self.hide_from_alt_tab,
        )
        .map_err(|e| Error::Create(e))?;
        winapi::debug_log(&format!("overlay: overlay hwnd=0x{:x}", hwnd.0 as usize));

        let mut rect = RECT::default();
        unsafe { GetClientRect(hwnd, &mut rect) };
        let width = (rect.right - rect.left).max(1) as u32;
        let height = (rect.bottom - rect.top).max(1) as u32;

        let mut d3d = d3d::D3D::new(width, height).map_err(|e| Error::Create(e))?;
        winapi::debug_log(&format!("overlay: d3d initialized {}x{}", width, height));
        let mut comp = dcomp::Composition::new(hwnd, &d3d).map_err(|e| Error::Create(e))?;
        comp.bind_swap_chain(&d3d).map_err(|e| Error::Create(e))?;
        winapi::debug_log("overlay: composition bound to swapchain");

        win::apply_transparency(hwnd);
        win::set_topmost(hwnd);
        if self.click_through_on_start {
            win::set_click_through(hwnd, true);
        } else {
            win::set_click_through(hwnd, false);
        }
        win::show_no_activate(hwnd);
        winapi::debug_log("overlay: window shown (no activate, topmost)");

        let egui_ctx = egui::Context::default();
        let mut input = InputCollector::new(hwnd);
        let mut painter = painter_d3d::PainterD3D::new(&d3d).map_err(|e| Error::Create(e))?;

        let mut click_through = self.click_through_on_start;
        let mut prev_toggle = false;

        let mut last_log = std::time::Instant::now();
        let start_time = std::time::Instant::now();
        loop {
            if !drain_messages(hwnd, &mut input) {
                break;
            }

            let raw = input.build_raw(width as f32, height as f32, start_time);
            egui_ctx.begin_pass(raw);
            app.ui(&egui_ctx);
            let out = egui_ctx.end_pass();
            let clipped = egui_ctx.tessellate(out.shapes.clone(), egui_ctx.pixels_per_point());
            winapi::debug_log(&format!(
                "overlay: shapes={} clipped={} tex_set={} tex_free={}",
                out.shapes.len(),
                clipped.len(),
                out.textures_delta.set.len(),
                out.textures_delta.free.len()
            ));

            painter
                .update_textures(&out.textures_delta)
                .map_err(|e| Error::Run(e))?;
            d3d.begin_frame();
            painter
                .paint(width, height, &clipped)
                .map_err(|e| Error::Run(e))?;
            d3d.present();
            if last_log.elapsed().as_secs_f32() > 1.0 {
                winapi::debug_log("overlay: frame");
                last_log = std::time::Instant::now();
            }

            let down = winapi::is_vk_pressed(self.toggle_vk);
            if down && !prev_toggle {
                click_through = !click_through;
                win::set_click_through(hwnd, click_through);
            }
            prev_toggle = down;

            if crate::SHUTDOWN.load(core::sync::atomic::Ordering::SeqCst) || winapi::is_f10_pressed() {
                unsafe { PostQuitMessage(0) };
                break;
            }

            if resize_if_needed(hwnd, &mut d3d, &mut painter) {
                let mut r = RECT::default();
                unsafe { GetClientRect(hwnd, &mut r) };
                let w = (r.right - r.left).max(1) as u32;
                let h = (r.bottom - r.top).max(1) as u32;
                input.set_screen(w as f32, h as f32);
            }
            unsafe {
                windows::Win32::System::Threading::Sleep(16);
            }
        }

        unsafe { DestroyWindow(hwnd); }
        unsafe { CoUninitialize(); }

        Ok(())
    }
}

fn drain_messages(hwnd: HWND, input: &mut InputCollector) -> bool {
    unsafe {
        let mut msg = MaybeUninit::<MSG>::zeroed();
        while PeekMessageW(msg.as_mut_ptr(), Some(HWND(null_mut())), 0, 0, PM_REMOVE).as_bool() {
            let m = msg.assume_init();
            if m.message == WM_QUIT {
                return false;
            }
            input.on_message(&m);
            TranslateMessage(&m);
            DispatchMessageW(&m);
        }
    }
    true
}

fn resize_if_needed(hwnd: HWND, d3d: &mut d3d::D3D, painter: &mut painter_d3d::PainterD3D) -> bool {
    let mut rect = RECT::default();
    unsafe { GetClientRect(hwnd, &mut rect) };
    let w = (rect.right - rect.left).max(1) as u32;
    let h = (rect.bottom - rect.top).max(1) as u32;
    if w != d3d.width || h != d3d.height {
        d3d.resize(w, h).ok();
        painter.on_resize(d3d).ok();
        return true;
    }
    false
}

struct InputCollector {
    hwnd: HWND,
    screen_w: f32,
    screen_h: f32,
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
}

impl InputCollector {
    fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            screen_w: 0.0,
            screen_h: 0.0,
            events: Vec::new(),
            modifiers: egui::Modifiers::default(),
        }
    }
    fn set_screen(&mut self, w: f32, h: f32) {
        self.screen_w = w;
        self.screen_h = h;
    }
    fn build_raw(&mut self, w: f32, h: f32, now: std::time::Instant) -> egui::RawInput {
        self.screen_w = w;
        self.screen_h = h;
        let screen_rect = egui::Rect::from_min_size(egui::Pos2::new(0.0, 0.0), egui::vec2(w, h));
        let events = std::mem::take(&mut self.events);
        egui::RawInput { screen_rect: Some(screen_rect), time: Some(now.elapsed().as_secs_f64()), max_texture_side: Some(8192), events, ..Default::default() }
    }
    fn on_message(&mut self, msg: &MSG) {
        match msg.message {
            WM_MOUSEMOVE => {
                let x = get_x_lparam(msg.lParam) as i32 as f32;
                let y = get_y_lparam(msg.lParam) as i32 as f32;
                self.events
                    .push(egui::Event::PointerMoved(egui::pos2(x, y)));
            }
            WM_LBUTTONDOWN => self.events.push(egui::Event::PointerButton {
                pos: pos_from_lparam(msg.lParam),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: self.modifiers,
            }),
            WM_LBUTTONUP => self.events.push(egui::Event::PointerButton {
                pos: pos_from_lparam(msg.lParam),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: self.modifiers,
            }),
            WM_RBUTTONDOWN => self.events.push(egui::Event::PointerButton {
                pos: pos_from_lparam(msg.lParam),
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: self.modifiers,
            }),
            WM_RBUTTONUP => self.events.push(egui::Event::PointerButton {
                pos: pos_from_lparam(msg.lParam),
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: self.modifiers,
            }),
            WM_MOUSEWHEEL => {
                let delta =
                    ((msg.wParam.0 as u32 >> 16) & 0xFFFF) as i16 as f32 / WHEEL_DELTA as f32;
                self.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -delta * 48.0),
                    modifiers: self.modifiers,
                });
            }
            WM_MOUSEHWHEEL => {
                let delta =
                    ((msg.wParam.0 as u32 >> 16) & 0xFFFF) as i16 as f32 / WHEEL_DELTA as f32;
                self.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(delta * 48.0, 0.0),
                    modifiers: self.modifiers,
                });
            }
            WM_KEYDOWN | WM_SYSKEYDOWN => {
                self.update_modifiers();
                if let Some(k) = vk_to_key(msg.wParam.0 as u32) {
                    self.events.push(egui::Event::Key {
                        key: k,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: self.modifiers,
                    });
                }
            }
            WM_KEYUP | WM_SYSKEYUP => {
                self.update_modifiers();
                if let Some(k) = vk_to_key(msg.wParam.0 as u32) {
                    self.events.push(egui::Event::Key {
                        key: k,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers: self.modifiers,
                    });
                }
            }
            WM_CHAR => {
                let ch = std::char::from_u32(msg.wParam.0 as u32).unwrap_or('\u{0}');
                if !ch.is_control() {
                    self.events.push(egui::Event::Text(ch.to_string()));
                }
            }
            WM_DESTROY => unsafe { PostQuitMessage(0) },
            _ => {}
        }
    }
    fn update_modifiers(&mut self) {
        unsafe {
            let ctrl = (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            let alt = (GetKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0;
            let shift = (GetKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000) != 0;
            let mac_cmd = false;
            self.modifiers = egui::Modifiers {
                alt,
                ctrl,
                shift,
                mac_cmd,
                command: ctrl,
            };
        }
    }
}

fn vk_to_key(vk: u32) -> Option<egui::Key> {
    match vk {
        0x25 => Some(egui::Key::ArrowLeft),
        0x26 => Some(egui::Key::ArrowUp),
        0x27 => Some(egui::Key::ArrowRight),
        0x28 => Some(egui::Key::ArrowDown),
        0x08 => Some(egui::Key::Backspace),
        0x2E => Some(egui::Key::Delete),
        0x0D => Some(egui::Key::Enter),
        0x1B => Some(egui::Key::Escape),
        0x09 => Some(egui::Key::Tab),
        _ => None,
    }
}

fn pos_from_lparam(lp: LPARAM) -> egui::Pos2 {
    egui::pos2(
        get_x_lparam(lp) as i32 as f32,
        get_y_lparam(lp) as i32 as f32,
    )
}
