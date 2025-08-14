use crate::overlay::OverlayBuilder;

#[derive(Clone, Debug)]
pub struct Config {
    // Template metadata
    pub project_name: String,
    pub game_name: String,
    pub author: String,
    pub version: String,

    // Overlay window
    pub window_title: String,
    pub always_on_top: bool,
    pub transparent: bool,
    pub decorated: bool,
    pub resizable: bool,
    pub fullscreen: bool,
    pub hide_from_alt_tab: bool,
    pub show_indicator: bool,

    // Hotkeys
    pub toggle_vk: i32, // VK code for click-through toggle
    pub exit_vk: i32,   // VK code to terminate runtime
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project_name: "Overlay Template".to_string(),
            game_name: "Your Game".to_string(),
            author: "Your Name".to_string(),
            version: "0.1.0".to_string(),
            window_title: "Overlay".to_string(),
            always_on_top: true,
            transparent: true,
            decorated: false,
            resizable: false,
            fullscreen: true,
            hide_from_alt_tab: true,
            show_indicator: true,
            toggle_vk: windows::Win32::UI::Input::KeyboardAndMouse::VK_INSERT.0 as _,
            exit_vk: windows::Win32::UI::Input::KeyboardAndMouse::VK_F10.0 as _,
        }
    }
}

impl Config {
    pub fn overlay_builder(&self) -> OverlayBuilder {
        OverlayBuilder::new()
            .title(if self.window_title.is_empty() {
                self.project_name.clone()
            } else {
                self.window_title.clone()
            })
            .hide_from_alt_tab(self.hide_from_alt_tab)
            .click_through(true)
            .toggle_key(self.toggle_vk)
    }
}
