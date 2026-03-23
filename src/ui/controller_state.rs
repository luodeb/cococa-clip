use cocoa::base::{BOOL, NO, YES, id, nil};
use cocoa::foundation::NSString;
use crate::hotkey;
use crate::ui::layout;
use crate::ui::widgets;
use objc::runtime::Object;
use objc::msg_send;
use std::cell::RefCell;

pub const HOTKEY_HINT_IDLE: &str = "准备好后点击“开始录制”";
pub const HOTKEY_HINT_RECORDING: &str = "正在录制中，按下组合键（Esc 可取消）";
pub const HOTKEY_HINT_PENDING: &str = "已捕获新组合，点击保存立即生效";

thread_local! {
    static HOTKEY_DRAFT_BINDING: RefCell<Option<hotkey::HotKeyBinding>> = RefCell::new(None);
}

pub fn set_hotkey_draft(binding: Option<hotkey::HotKeyBinding>) {
    HOTKEY_DRAFT_BINDING.with(|slot| {
        *slot.borrow_mut() = binding;
    });
}

pub fn hotkey_draft() -> Option<hotkey::HotKeyBinding> {
    HOTKEY_DRAFT_BINDING.with(|slot| *slot.borrow())
}

pub fn refresh_hotkey_views(this: &Object) {
    let footer = footer_hotkey_label(this);
    let preview = settings_preview_label(this);

    let current = hotkey::current_binding();
    let next = hotkey_draft().unwrap_or(current);

    widgets::set_label_text(footer, &current.preview_text());
    widgets::set_label_text(preview, &next.preview_text());
}

pub fn set_recording_state(this: &Object, recording: bool, hint: Option<&str>) {
    unsafe {
        let this_mut = this as *const Object as *mut Object;
        (*this_mut).set_ivar("hotkey_recording", if recording { YES } else { NO });

        let hint_label = settings_hint_label(this);
        let record_button = settings_record_button(this);
        let save_button = settings_save_button(this);
        let cancel_button = settings_cancel_button(this);
        let has_draft = hotkey_draft().is_some();

        let text = hint.unwrap_or(if recording {
            HOTKEY_HINT_RECORDING
        } else if has_draft {
            HOTKEY_HINT_PENDING
        } else {
            HOTKEY_HINT_IDLE
        });

        widgets::set_label_text(hint_label, text);

        let _: () = msg_send![record_button, setHidden: if recording || has_draft { YES } else { NO }];
        let _: () = msg_send![save_button, setHidden: if has_draft { NO } else { YES }];
        let _: () = msg_send![cancel_button, setHidden: if recording || has_draft { NO } else { YES }];
        let _: () = msg_send![save_button, setEnabled: if has_draft { YES } else { NO }];
    }
}

pub fn hide_settings_window(this: &Object) {
    unsafe {
        let settings_window = settings_window(this);
        if settings_window != nil {
            let _: () = msg_send![settings_window, orderOut: nil];
        }
        let this_mut = this as *const Object as *mut Object;
        (*this_mut).set_ivar("settings_visible", NO);
        set_hotkey_draft(None);
        set_recording_state(this, false, None);
        refresh_hotkey_views(this);
    }
}

pub fn clear_input(input_field: id) {
    unsafe {
        let empty = NSString::alloc(nil).init_str("");
        let _: () = msg_send![input_field, setStringValue: empty];
    }
}

pub fn controller_from_window(window: id) -> id {
    unsafe {
        if window == nil {
            return nil;
        }
        msg_send![window, delegate]
    }
}

pub fn is_settings_visible(this: &Object) -> bool {
    unsafe {
        let value: BOOL = *this.get_ivar("settings_visible");
        value == YES
    }
}

pub fn is_hotkey_recording(this: &Object) -> bool {
    unsafe {
        let value: BOOL = *this.get_ivar("hotkey_recording");
        value == YES
    }
}

pub fn set_settings_visible(this: &Object, visible: bool) {
    unsafe {
        let this_mut = this as *const Object as *mut Object;
        (*this_mut).set_ivar("settings_visible", if visible { YES } else { NO });
    }
}

pub fn main_window(this: &Object) -> id {
    unsafe { *this.get_ivar("main_window") }
}

pub fn settings_window(this: &Object) -> id {
    unsafe { *this.get_ivar("settings_window") }
}

pub fn history_document(this: &Object) -> id {
    unsafe { *this.get_ivar("history_document") }
}

pub fn input_field(this: &Object) -> id {
    unsafe {
        let direct: id = *this.get_ivar("input_field");
        if direct != nil {
            return direct;
        }
        layout::locate_input_field(main_window(this))
    }
}

pub fn footer_hotkey_label(this: &Object) -> id {
    unsafe { *this.get_ivar("footer_hotkey_label") }
}

pub fn settings_hint_label(this: &Object) -> id {
    unsafe { *this.get_ivar("settings_hint_label") }
}

pub fn settings_preview_label(this: &Object) -> id {
    unsafe { *this.get_ivar("settings_preview_label") }
}

pub fn settings_record_button(this: &Object) -> id {
    unsafe { *this.get_ivar("settings_record_button") }
}

pub fn settings_save_button(this: &Object) -> id {
    unsafe { *this.get_ivar("settings_save_button") }
}

pub fn settings_cancel_button(this: &Object) -> id {
    unsafe { *this.get_ivar("settings_cancel_button") }
}
