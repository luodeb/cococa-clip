use cocoa::base::id;

use crate::ui::layout_main;
use crate::ui::layout_settings;

pub const MAIN_WIDTH: f64 = 400.0;
pub const MAIN_HEIGHT: f64 = 744.0;
pub const SETTINGS_WIDTH: f64 = 420.0;
pub const SETTINGS_HEIGHT: f64 = 420.0;

pub struct UiHandles {
    pub main_window: id,
    pub settings_window: id,
    pub history_document: id,
    pub input_field: id,
    pub footer_hotkey_label: id,
    pub settings_hint_label: id,
    pub settings_preview_label: id,
    pub settings_record_button: id,
    pub settings_save_button: id,
    pub settings_cancel_button: id,
    pub settings_autostart_toggle: id,
}

pub fn build_windows(controller: id) -> UiHandles {
    unsafe {
        let main_window = layout_main::build_main_window(controller, MAIN_WIDTH, MAIN_HEIGHT);
        let settings_window =
            layout_settings::build_settings_window(controller, main_window, SETTINGS_WIDTH, SETTINGS_HEIGHT);

        let main_content: id = objc::msg_send![main_window, contentView];
        let settings_content: id = objc::msg_send![settings_window, contentView];

        let history_document = layout_main::build_history_section(main_content, MAIN_WIDTH, MAIN_HEIGHT);
        let input_field = layout_main::build_input_section(main_content, controller, MAIN_WIDTH, MAIN_HEIGHT);
        let footer_hotkey_label = layout_main::build_footer(main_content, MAIN_WIDTH);

        let (settings_hint_label, settings_preview_label, settings_record_button, settings_save_button, settings_cancel_button, settings_autostart_toggle) =
            layout_settings::build_settings_form(settings_content, controller, SETTINGS_WIDTH, SETTINGS_HEIGHT);

        UiHandles {
            main_window,
            settings_window,
            history_document,
            input_field,
            footer_hotkey_label,
            settings_hint_label,
            settings_preview_label,
            settings_record_button,
            settings_save_button,
            settings_cancel_button,
            settings_autostart_toggle,
        }
    }
}

pub fn locate_input_field(main_window: id) -> id {
    layout_main::locate_input_field(main_window)
}

pub fn place_settings_window(main_window: id, settings_window: id) {
    layout_settings::place_settings_window(main_window, settings_window, SETTINGS_WIDTH, SETTINGS_HEIGHT)
}

pub fn is_window_visible(window: id) -> bool {
    layout_main::is_window_visible(window)
}
