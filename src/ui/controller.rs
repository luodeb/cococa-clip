use cocoa::base::{BOOL, NO, YES, id, nil};
use log::error;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Once;
use std::thread;
use std::time::Duration;

use crate::app;
use crate::history;
use crate::hotkey;
use crate::hotkey::HotKeyCommand;
use crate::paste;
use crate::tray;
use crate::tray::TrayCommand;
use crate::ui::controller_events;
use crate::ui::controller_state;
use crate::ui::history_list;
use crate::ui::layout;

const PANEL_HIDE_BEFORE_PASTE_MS: u64 = 40;
const TRAY_POLL_INTERVAL_SECONDS: f64 = 0.1;

pub fn new_controller_instance() -> id {
    unsafe {
        let cls = register_controller_class();
        msg_send![cls, new]
    }
}

fn register_controller_class() -> *const Class {
    static ONCE: Once = Once::new();
    static mut CLASS: *const Class = std::ptr::null();

    ONCE.call_once(|| unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("ClipUiController", superclass)
            .expect("ClipUiController class declaration failed");

        decl.add_ivar::<id>("main_window");
        decl.add_ivar::<id>("settings_window");
        decl.add_ivar::<id>("history_document");
        decl.add_ivar::<id>("input_field");
        decl.add_ivar::<id>("footer_hotkey_label");
        decl.add_ivar::<id>("settings_hint_label");
        decl.add_ivar::<id>("settings_preview_label");
        decl.add_ivar::<id>("settings_record_button");
        decl.add_ivar::<id>("settings_save_button");
        decl.add_ivar::<id>("settings_cancel_button");
        decl.add_ivar::<id>("global_click_monitor");
        decl.add_ivar::<id>("local_click_monitor");
        decl.add_ivar::<id>("local_key_monitor");
        decl.add_ivar::<BOOL>("settings_visible");
        decl.add_ivar::<BOOL>("hotkey_recording");

        decl.add_method(
            sel!(applicationDidFinishLaunching:),
            did_finish_launching as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(applicationShouldTerminateAfterLastWindowClosed:),
            should_terminate_after_last_window_closed as extern "C" fn(&Object, Sel, id) -> BOOL,
        );
        decl.add_method(sel!(submitText:), submit_text as extern "C" fn(&Object, Sel, id));
        decl.add_method(
            sel!(openSettings:),
            open_settings as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(closeSettings:),
            close_settings as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(beginRecord:),
            begin_record as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(sel!(saveRecord:), save_record as extern "C" fn(&Object, Sel, id));
        decl.add_method(
            sel!(cancelRecord:),
            cancel_record as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(historyRowPressed:),
            history_row_pressed as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(sel!(pollTick:), poll_tick as extern "C" fn(&Object, Sel, id));

        CLASS = decl.register();
    });

    unsafe { CLASS }
}

extern "C" fn did_finish_launching(this: &Object, _: Sel, _: id) {
    run_ffi_void("applicationDidFinishLaunching", || unsafe {
        let handles = layout::build_windows(this as *const Object as id);

        let this_mut = this as *const Object as *mut Object;
        (*this_mut).set_ivar("main_window", handles.main_window);
        (*this_mut).set_ivar("settings_window", handles.settings_window);
        (*this_mut).set_ivar("history_document", handles.history_document);
        (*this_mut).set_ivar("input_field", handles.input_field);
        (*this_mut).set_ivar("footer_hotkey_label", handles.footer_hotkey_label);
        (*this_mut).set_ivar("settings_hint_label", handles.settings_hint_label);
        (*this_mut).set_ivar("settings_preview_label", handles.settings_preview_label);
        (*this_mut).set_ivar("settings_record_button", handles.settings_record_button);
        (*this_mut).set_ivar("settings_save_button", handles.settings_save_button);
        (*this_mut).set_ivar("settings_cancel_button", handles.settings_cancel_button);
        (*this_mut).set_ivar("settings_visible", NO);
        (*this_mut).set_ivar("hotkey_recording", NO);

        app::keep_panel_above_apps(handles.main_window);
        app::keep_panel_above_apps(handles.settings_window);

        let _: () = msg_send![handles.main_window, center];
        layout::place_settings_window(handles.main_window, handles.settings_window);
        controller_events::install_event_monitors(
            this_mut,
            handles.main_window,
            handles.settings_window,
        );

        if let Err(err) = tray::init_tray() {
            error!("初始化托盘失败: {err}");
        } else {
            schedule_poll(this as *const Object as id);
        }

        if let Err(err) = hotkey::init_hotkey() {
            error!("初始化全局快捷键失败: {err}");
        }

        if let Err(err) = history::init_history() {
            error!("初始化剪切板历史失败: {err}");
        }

        controller_state::refresh_hotkey_views(this);
        render_history(this);
        show_main_window(this);
    });
}

fn run_ffi_void(name: &str, callback: impl FnOnce()) {
    if let Err(payload) = catch_unwind(AssertUnwindSafe(callback)) {
        let panic_message = if let Some(message) = payload.downcast_ref::<&str>() {
            (*message).to_owned()
        } else if let Some(message) = payload.downcast_ref::<String>() {
            message.clone()
        } else {
            "<non-string panic payload>".to_owned()
        };

        error!("{name} panic: {panic_message}");
    }
}

extern "C" fn should_terminate_after_last_window_closed(_: &Object, _: Sel, _: id) -> BOOL {
    NO
}

extern "C" fn submit_text(this: &Object, _: Sel, _: id) {
    unsafe {
        let main_window = controller_state::main_window(this);
        let input_field = controller_state::input_field(this);
        if main_window == nil || input_field == nil {
            return;
        }

        let value: id = msg_send![input_field, stringValue];
        let c_str_ptr: *const c_char = msg_send![value, UTF8String];
        if c_str_ptr.is_null() {
            return;
        }
        let value_text = CStr::from_ptr(c_str_ptr).to_string_lossy();

        let _: () = msg_send![main_window, orderOut: nil];
        thread::sleep(Duration::from_millis(PANEL_HIDE_BEFORE_PASTE_MS));

        if let Err(err) = paste::commit_text(&value_text) {
            error!("粘贴流程失败: {err}");
        }

        controller_state::clear_input(input_field);
    }
}

extern "C" fn open_settings(this: &Object, _: Sel, _: id) {
    unsafe {
        let settings_window = controller_state::settings_window(this);
        let main_window = controller_state::main_window(this);
        if settings_window == nil || main_window == nil {
            return;
        }

        layout::place_settings_window(main_window, settings_window);
        let _: () = msg_send![settings_window, orderFrontRegardless];
        let _: () = msg_send![settings_window, makeKeyWindow];

        controller_state::set_settings_visible(this, true);
        controller_state::refresh_hotkey_views(this);
    }
}

extern "C" fn close_settings(this: &Object, _: Sel, _: id) {
    controller_state::hide_settings_window(this);
}

extern "C" fn begin_record(this: &Object, _: Sel, _: id) {
    controller_state::set_hotkey_draft(None);
    controller_state::set_recording_state(this, true, None);
    controller_state::refresh_hotkey_views(this);
}

extern "C" fn save_record(this: &Object, _: Sel, _: id) {
    let Some(binding) = controller_state::hotkey_draft() else {
        return;
    };

    match hotkey::set_binding(binding) {
        Ok(_) => {
            controller_state::set_hotkey_draft(None);
            controller_state::set_recording_state(this, false, None);
            controller_state::refresh_hotkey_views(this);
        }
        Err(err) => {
            error!("更新全局快捷键失败: {err}");
            controller_state::set_recording_state(this, false, Some("系统未接受该快捷键，请换一个组合"));
        }
    }
}

extern "C" fn cancel_record(this: &Object, _: Sel, _: id) {
    controller_state::set_hotkey_draft(None);
    controller_state::set_recording_state(this, false, None);
    controller_state::refresh_hotkey_views(this);
}

extern "C" fn history_row_pressed(this: &Object, _: Sel, sender: id) {
    unsafe {
        let main_window = controller_state::main_window(this);
        if main_window == nil || sender == nil {
            return;
        }

        let entry_id: isize = msg_send![sender, tag];
        if entry_id <= 0 {
            return;
        }

        let _: () = msg_send![main_window, orderOut: nil];
        thread::sleep(Duration::from_millis(PANEL_HIDE_BEFORE_PASTE_MS));

        if let Err(err) = history::paste_entry(entry_id as i64) {
            error!("历史条目粘贴失败: {err}");
        }
    }
}

extern "C" fn poll_tick(this: &Object, _: Sel, _: id) {
    let main_window = controller_state::main_window(this);

    match tray::poll_command() {
        Some(TrayCommand::ShowWindow) => show_main_window(this),
        Some(TrayCommand::Quit) => app::terminate_app(),
        None => {}
    }

    match hotkey::poll_command() {
        Some(HotKeyCommand::ShowWindow) => show_main_window(this),
        None => {}
    }

    match history::poll_clipboard_change() {
        Ok(true) => {
            if main_window != nil {
                render_history(this);
            }
        }
        Ok(false) => {}
        Err(err) => error!("轮询剪切板历史失败: {err}"),
    }
}

fn schedule_poll(controller: id) {
    unsafe {
        let _: id = msg_send![
            class!(NSTimer),
            scheduledTimerWithTimeInterval: TRAY_POLL_INTERVAL_SECONDS
            target: controller
            selector: sel!(pollTick:)
            userInfo: nil
            repeats: YES
        ];
    }
}

fn render_history(this: &Object) {
    let document_view = controller_state::history_document(this);
    history_list::render(document_view, this as *const Object as id);
}

fn show_main_window(this: &Object) {
    unsafe {
        let main_window = controller_state::main_window(this);
        if main_window == nil {
            return;
        }

        let _: () = msg_send![main_window, orderFrontRegardless];
        let _: () = msg_send![main_window, makeKeyWindow];

        let input = controller_state::input_field(this);
        if input == nil {
            return;
        }

        let _: BOOL = msg_send![main_window, makeFirstResponder: input];
        let _: () = msg_send![input, selectText: nil];
    }
}
