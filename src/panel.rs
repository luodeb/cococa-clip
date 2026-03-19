use block::ConcreteBlock;
use cocoa::appkit::NSBackingStoreType;
use cocoa::appkit::NSEventMask;
use cocoa::base::{BOOL, NO, YES, id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
use log::{debug, error};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel};
use std::cell::RefCell;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Once;
use std::thread;
use std::time::Duration;

use crate::app;
use crate::history;
use crate::history::HistoryEntry;
use crate::hotkey;
use crate::hotkey::HotKeyCommand;
use crate::icons;
use crate::paste;
use crate::tray;
use crate::tray::TrayCommand;

const NS_WINDOW_STYLE_TITLED: usize = 1 << 0;
const NS_WINDOW_STYLE_CLOSABLE: usize = 1 << 1;
const NS_WINDOW_STYLE_NONACTIVATING_PANEL: usize = 1 << 7;
const NS_WINDOW_STYLE_FULL_SIZE_CONTENT_VIEW: usize = 1 << 15;

const NS_WINDOW_BUTTON_CLOSE: usize = 0;
const NS_WINDOW_BUTTON_MINIMIZE: usize = 1;
const NS_WINDOW_BUTTON_ZOOM: usize = 2;
const NS_WINDOW_TITLE_HIDDEN: usize = 1;
const NS_FOCUS_RING_TYPE_NONE: usize = 1;

const NS_TRACKING_MOUSE_ENTERED_AND_EXITED: usize = 0x01;
const NS_TRACKING_ACTIVE_ALWAYS: usize = 0x80;
const NS_TRACKING_IN_VISIBLE_RECT: usize = 0x200;

const PANEL_WIDTH: f64 = 430.0;
const PANEL_HEIGHT: f64 = 656.0;
const PANEL_RADIUS: f64 = 26.0;
const PANEL_SIDE_MARGIN: f64 = 18.0;

const INPUT_TAG: isize = 1001;

const PANEL_HIDE_BEFORE_PASTE_MS: u64 = 40;
const TRAY_POLL_INTERVAL_SECONDS: f64 = 0.1;
const HISTORY_LIMIT: usize = 60;

const HEADER_BUTTON_SIZE: f64 = 30.0;
const HEADER_BUTTON_SPACING: f64 = 16.0;
const HEADER_ICON_SIZE: u32 = 22;
const SETTINGS_PAGE_OFFSET_X: f64 = PANEL_WIDTH + 24.0;

const SETTINGS_TITLE_Y: f64 = PANEL_HEIGHT - 148.0;
const SETTINGS_SUBTITLE_Y: f64 = PANEL_HEIGHT - 182.0;
const SETTINGS_CARD_WIDTH: f64 = PANEL_WIDTH - PANEL_SIDE_MARGIN * 2.0;
const SETTINGS_PREVIEW_Y: f64 = PANEL_HEIGHT - 436.0;
const SETTINGS_PREVIEW_HEIGHT: f64 = 156.0;
const SETTINGS_ACTION_BUTTON_Y: f64 = 74.0;
const SETTINGS_ACTION_BUTTON_HEIGHT: f64 = 44.0;
const SETTINGS_ACTION_BUTTON_GAP: f64 = 12.0;
const SETTINGS_ACTION_BUTTON_WIDTH: f64 = (SETTINGS_CARD_WIDTH - SETTINGS_ACTION_BUTTON_GAP) / 2.0;

const HOTKEY_SETTINGS_IDLE_HINT: &str = "点击“录制新快捷键”后按下组合键，确认无误再保存";
const HOTKEY_SETTINGS_RECORDING_HINT: &str = "请按下新的快捷键组合，按 Esc 或点“取消”放弃修改";
const HOTKEY_SETTINGS_PENDING_HINT: &str = "新的快捷键已录制，点击“保存”应用，或点“取消”保留原绑定";
const HOTKEY_CAPTURE_CANCEL_KEYCODE: u16 = 53;

const SEARCH_BAR_HEIGHT: f64 = 56.0;
const SEARCH_BAR_Y: f64 = PANEL_HEIGHT - 108.0;
const SEARCH_ICON_SIZE: u32 = 21;
const SEARCH_RIGHT_BUTTON_SIZE: f64 = 34.0;
const SEARCH_RIGHT_ICON_SIZE: u32 = 18;

const DIVIDER_Y: f64 = SEARCH_BAR_Y - 18.0;
const FOOTER_HEIGHT: f64 = 62.0;
const HISTORY_SCROLL_Y: f64 = FOOTER_HEIGHT;
const HISTORY_SCROLL_HEIGHT: f64 = DIVIDER_Y - HISTORY_SCROLL_Y;
const HISTORY_ROW_HEIGHT: f64 = 86.0;
const HISTORY_ICON_BOX_SIZE: f64 = 40.0;
const HISTORY_ICON_SIZE: u32 = 18;

const SETTINGS_ICON_SVG: &str = include_str!("../assets/icons/settings.svg");
const DELETE_ICON_SVG: &str = include_str!("../assets/icons/trash.svg");
const SEARCH_ICON_SVG: &str = include_str!("../assets/icons/search.svg");
const FILTER_ICON_SVG: &str = include_str!("../assets/icons/filter.svg");
const LIST_ICON_SVG: &str = include_str!("../assets/icons/list.svg");

thread_local! {
    static HOTKEY_DRAFT_BINDING: RefCell<Option<hotkey::HotKeyBinding>> = RefCell::new(None);
}

pub fn register_controller_class() -> *const Class {
    static ONCE: Once = Once::new();
    static mut CLASS: *const Class = std::ptr::null();

    ONCE.call_once(|| unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("ClipPanelController", superclass)
            .expect("ClipPanelController class declaration failed");
        decl.add_ivar::<id>("panel");
        decl.add_ivar::<id>("history_document_view");
        decl.add_ivar::<id>("global_click_monitor");
        decl.add_ivar::<id>("local_click_monitor");
        decl.add_ivar::<id>("local_key_monitor");
        decl.add_ivar::<id>("local_scroll_monitor");
        decl.add_ivar::<id>("main_page");
        decl.add_ivar::<id>("settings_page");
        decl.add_ivar::<id>("settings_subtitle_label");
        decl.add_ivar::<id>("footer_modifier_label");
        decl.add_ivar::<id>("footer_key_label");
        decl.add_ivar::<id>("settings_preview_label");
        decl.add_ivar::<id>("settings_record_button");
        decl.add_ivar::<id>("settings_save_button");
        decl.add_ivar::<id>("settings_cancel_button");
        decl.add_ivar::<BOOL>("hotkey_recording");
        decl.add_ivar::<BOOL>("settings_visible");

        decl.add_method(
            sel!(applicationDidFinishLaunching:),
            did_finish_launching as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(applicationShouldTerminateAfterLastWindowClosed:),
            should_terminate_after_last_window_closed as extern "C" fn(&Object, Sel, id) -> BOOL,
        );
        decl.add_method(
            sel!(windowWillClose:),
            window_will_close as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(submitText:),
            submit_text as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(clearInput:),
            clear_input as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(showSettings:),
            show_settings as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(beginHotkeyRecording:),
            begin_hotkey_recording as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(saveHotkeyBinding:),
            save_hotkey_binding as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(cancelHotkeyRecording:),
            cancel_hotkey_recording as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(historyEntrySelected:),
            history_entry_selected as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(pollTray:),
            poll_tray as extern "C" fn(&Object, Sel, id),
        );

        CLASS = decl.register();
    });

    unsafe { CLASS }
}

fn register_history_row_button_class() -> *const Class {
    static ONCE: Once = Once::new();
    static mut CLASS: *const Class = std::ptr::null();

    ONCE.call_once(|| unsafe {
        let superclass = class!(NSButton);
        let mut decl = ClassDecl::new("ClipHistoryRowButton", superclass)
            .expect("ClipHistoryRowButton class declaration failed");
        decl.add_ivar::<id>("hover_overlay");

        decl.add_method(
            sel!(mouseEntered:),
            history_row_mouse_entered as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(mouseExited:),
            history_row_mouse_exited as extern "C" fn(&Object, Sel, id),
        );

        CLASS = decl.register();
    });

    unsafe { CLASS }
}

pub fn new_controller_instance() -> id {
    unsafe {
        let cls = register_controller_class();
        let controller: id = msg_send![cls, new];
        controller
    }
}

extern "C" fn did_finish_launching(this: &Object, _: Sel, _: id) {
    unsafe {
        let panel = build_panel(this as *const Object as id);

        let this_mut = this as *const Object as *mut Object;
        (*this_mut).set_ivar("panel", panel);
        install_outside_click_monitors(this_mut, panel);

        if let Err(err) = tray::init_tray() {
            error!("初始化托盘失败: {err}");
        } else {
            schedule_tray_poll(this as *const Object as id);
        }

        if let Err(err) = hotkey::init_hotkey() {
            error!("初始化全局快捷键失败: {err}");
        }

        if let Err(err) = history::init_history() {
            error!("初始化剪切板历史失败: {err}");
        }

        app::keep_panel_above_apps(panel);
        let _: () = msg_send![panel, center];

        refresh_hotkey_views(panel);
        render_history_entries(panel);
        show_panel(panel);
    }
}

extern "C" fn should_terminate_after_last_window_closed(_: &Object, _: Sel, _: id) -> BOOL {
    NO
}

extern "C" fn window_will_close(_: &Object, _: Sel, _: id) {
    debug!("panel closed, keep app alive for tray access");
}

extern "C" fn submit_text(this: &Object, _: Sel, _: id) {
    unsafe {
        let panel: id = *this.get_ivar("panel");
        if panel == nil {
            return;
        }

        let input_field = input_field_from_panel(panel);
        if input_field == nil {
            return;
        }

        let value: id = msg_send![input_field, stringValue];
        let c_str_ptr: *const c_char = msg_send![value, UTF8String];
        if c_str_ptr.is_null() {
            return;
        }

        let value_text = CStr::from_ptr(c_str_ptr).to_string_lossy();

        let _: () = msg_send![panel, orderOut: nil];
        thread::sleep(Duration::from_millis(PANEL_HIDE_BEFORE_PASTE_MS));

        if let Err(err) = paste::commit_text(&value_text) {
            error!("粘贴流程失败: {err}");
        }

        clear_input_field(input_field);
    }
}

extern "C" fn clear_input(this: &Object, _: Sel, _: id) {
    unsafe {
        let panel: id = *this.get_ivar("panel");
        if panel == nil {
            return;
        }

        let input_field = input_field_from_panel(panel);
        if input_field == nil {
            return;
        }

        clear_input_field(input_field);
        let _: BOOL = msg_send![panel, makeFirstResponder: input_field];
        let _: () = msg_send![input_field, selectText: nil];
    }
}

extern "C" fn show_settings(this: &Object, _: Sel, _: id) {
    unsafe {
        let panel: id = *this.get_ivar("panel");
        if panel == nil {
            return;
        }

        let visible: BOOL = *this.get_ivar("settings_visible");
        transition_settings_page(panel, visible == NO);
    }
}

extern "C" fn begin_hotkey_recording(this: &Object, _: Sel, _: id) {
    unsafe {
        let panel: id = *this.get_ivar("panel");
        if panel == nil {
            return;
        }

        let _: BOOL = msg_send![panel, makeFirstResponder: nil];
        set_hotkey_draft_binding(None);
        refresh_hotkey_views(panel);
        set_hotkey_recording(panel, true, None);
    }
}

extern "C" fn save_hotkey_binding(this: &Object, _: Sel, _: id) {
    unsafe {
        let panel: id = *this.get_ivar("panel");
        if panel == nil {
            return;
        }

        let Some(binding) = hotkey_draft_binding() else {
            return;
        };

        match hotkey::set_binding(binding) {
            Ok(_) => {
                set_hotkey_draft_binding(None);
                refresh_hotkey_views(panel);
                pulse_hotkey_labels(panel);
                set_hotkey_recording(panel, false, None);
            }
            Err(err) => {
                error!("更新全局快捷键失败: {err}");
                set_hotkey_recording(panel, false, Some("系统未接受该快捷键，请换一个组合"));
            }
        }
    }
}

extern "C" fn cancel_hotkey_recording(this: &Object, _: Sel, _: id) {
    unsafe {
        let panel: id = *this.get_ivar("panel");
        if panel == nil {
            return;
        }

        cancel_hotkey_recording_if_needed(panel);
    }
}

extern "C" fn history_entry_selected(this: &Object, _: Sel, sender: id) {
    unsafe {
        let panel: id = *this.get_ivar("panel");
        if panel == nil || sender == nil {
            return;
        }

        let entry_id: isize = msg_send![sender, tag];
        if entry_id <= 0 {
            return;
        }

        let _: () = msg_send![panel, orderOut: nil];
        thread::sleep(Duration::from_millis(PANEL_HIDE_BEFORE_PASTE_MS));

        if let Err(err) = history::paste_entry(entry_id as i64) {
            error!("历史条目粘贴失败: {err}");
        }
    }
}

extern "C" fn poll_tray(this: &Object, _: Sel, _: id) {
    unsafe {
        let panel: id = *this.get_ivar("panel");

        match tray::poll_command() {
            Some(TrayCommand::ShowWindow) => {
                if panel != nil {
                    show_panel(panel);
                }
            }
            Some(TrayCommand::Quit) => {
                app::terminate_app();
            }
            None => {}
        }

        match hotkey::poll_command() {
            Some(HotKeyCommand::ShowWindow) => {
                if panel != nil {
                    show_panel(panel);
                }
            }
            None => {}
        }

        match history::poll_clipboard_change() {
            Ok(true) => {
                if panel != nil {
                    render_history_entries(panel);
                }
            }
            Ok(false) => {}
            Err(err) => {
                error!("轮询剪切板历史失败: {err}");
            }
        }
    }
}

fn schedule_tray_poll(controller: id) {
    unsafe {
        let _: id = msg_send![
            class!(NSTimer),
            scheduledTimerWithTimeInterval: TRAY_POLL_INTERVAL_SECONDS
            target: controller
            selector: sel!(pollTray:)
            userInfo: nil
            repeats: YES
        ];
    }
}

fn show_panel(panel: id) {
    unsafe {
        let _: () = msg_send![panel, orderFrontRegardless];
        let _: () = msg_send![panel, makeKeyWindow];

        let controller = controller_from_panel(panel);
        if controller != nil {
            let controller = &*(controller as *const Object);
            let settings_visible: BOOL = *controller.get_ivar("settings_visible");
            if settings_visible == YES {
                let _: BOOL = msg_send![panel, makeFirstResponder: nil];
                return;
            }
        }

        let input_field = input_field_from_panel(panel);
        if input_field != nil {
            let _: BOOL = msg_send![panel, makeFirstResponder: input_field];
            let _: () = msg_send![input_field, selectText: nil];
        }
    }
}

fn install_outside_click_monitors(controller: *mut Object, panel: id) {
    unsafe {
        if controller.is_null() || panel == nil {
            return;
        }

        let mouse_down_mask = (NSEventMask::NSLeftMouseDownMask
            | NSEventMask::NSRightMouseDownMask
            | NSEventMask::NSOtherMouseDownMask)
            .bits();

        let global_handler = ConcreteBlock::new(move |_event: id| {
            cancel_hotkey_recording_if_needed(panel);
            hide_panel_if_outside_click(panel);
        })
        .copy();
        let global_monitor: id = msg_send![
            class!(NSEvent),
            addGlobalMonitorForEventsMatchingMask: mouse_down_mask
            handler: &*global_handler
        ];

        let local_handler = ConcreteBlock::new(move |event: id| -> id {
            cancel_hotkey_recording_if_needed(panel);
            hide_panel_if_outside_click(panel);
            event
        })
        .copy();
        let local_monitor: id = msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: mouse_down_mask
            handler: &*local_handler
        ];

        let key_handler = ConcreteBlock::new(move |event: id| -> id {
            if handle_hotkey_recording_key_event(panel, event) {
                return nil;
            }

            event
        })
        .copy();
        let key_monitor: id = msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: NSEventMask::NSKeyDownMask.bits()
            handler: &*key_handler
        ];

        let scroll_handler = ConcreteBlock::new(move |event: id| -> id {
            if should_block_scroll_event(panel, event) {
                return nil;
            }

            event
        })
        .copy();
        let scroll_monitor: id = msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: NSEventMask::NSScrollWheelMask.bits()
            handler: &*scroll_handler
        ];

        (*controller).set_ivar("global_click_monitor", global_monitor);
        (*controller).set_ivar("local_click_monitor", local_monitor);
        (*controller).set_ivar("local_key_monitor", key_monitor);
        (*controller).set_ivar("local_scroll_monitor", scroll_monitor);
    }
}

fn should_block_scroll_event(panel: id, event: id) -> bool {
    unsafe {
        if panel == nil || event == nil {
            return false;
        }

        let controller = controller_from_panel(panel);
        if controller == nil {
            return false;
        }

        let controller = &*(controller as *const Object);
        let settings_visible: BOOL = *controller.get_ivar("settings_visible");
        if settings_visible != YES {
            return false;
        }

        let is_visible: BOOL = msg_send![panel, isVisible];
        if is_visible != YES {
            return false;
        }

        let event_location_in_window: NSPoint = msg_send![event, locationInWindow];
        let event_location_on_screen: NSPoint = msg_send![panel, convertPointToScreen: event_location_in_window];

        point_inside_panel_frame(panel, event_location_on_screen)
    }
}

fn current_mouse_location() -> NSPoint {
    unsafe { msg_send![class!(NSEvent), mouseLocation] }
}

fn hide_panel_if_outside_click(panel: id) {
    unsafe {
        if panel == nil {
            return;
        }

        let is_visible: bool = msg_send![panel, isVisible];
        if !is_visible {
            return;
        }

        let click_location = current_mouse_location();
        if point_inside_panel_frame(panel, click_location) {
            return;
        }

        let _: () = msg_send![panel, orderOut: nil];
    }
}

fn point_inside_panel_frame(panel: id, point: NSPoint) -> bool {
    unsafe {
        let frame: NSRect = msg_send![panel, frame];
        point.x >= frame.origin.x
            && point.x <= frame.origin.x + frame.size.width
            && point.y >= frame.origin.y
            && point.y <= frame.origin.y + frame.size.height
    }
}

fn input_field_from_panel(panel: id) -> id {
    unsafe {
        let content_view: id = msg_send![panel, contentView];
        let input_field: id = msg_send![content_view, viewWithTag: INPUT_TAG];
        input_field
    }
}

fn history_document_view_from_panel(panel: id) -> id {
    unsafe {
        let controller = controller_from_panel(panel);
        if controller == nil {
            return nil;
        }

        let controller = &*(controller as *const Object);
        *controller.get_ivar("history_document_view")
    }
}

fn controller_from_panel(panel: id) -> id {
    unsafe {
        if panel == nil {
            return nil;
        }

        msg_send![panel, delegate]
    }
}

fn main_page_from_panel(panel: id) -> id {
    unsafe {
        let controller = controller_from_panel(panel);
        if controller == nil {
            return nil;
        }

        let controller = &*(controller as *const Object);
        *controller.get_ivar("main_page")
    }
}

fn settings_page_from_panel(panel: id) -> id {
    unsafe {
        let controller = controller_from_panel(panel);
        if controller == nil {
            return nil;
        }

        let controller = &*(controller as *const Object);
        *controller.get_ivar("settings_page")
    }
}

fn set_label_text(label: id, text: &str) {
    unsafe {
        if label == nil {
            return;
        }

        let value = NSString::alloc(nil).init_str(text);
        let _: () = msg_send![label, setStringValue: value];
    }
}

fn set_view_hidden(view: id, hidden: bool) {
    unsafe {
        if view == nil {
            return;
        }

        let _: () = msg_send![view, setHidden: if hidden { YES } else { NO }];
    }
}

fn set_button_enabled(button: id, enabled: bool) {
    unsafe {
        if button == nil {
            return;
        }

        let _: () = msg_send![button, setEnabled: if enabled { YES } else { NO }];
        let _: () = msg_send![button, setAlphaValue: if enabled { 1.0f64 } else { 0.56f64 }];
    }
}

fn hotkey_draft_binding() -> Option<hotkey::HotKeyBinding> {
    HOTKEY_DRAFT_BINDING.with(|draft| *draft.borrow())
}

fn set_hotkey_draft_binding(binding: Option<hotkey::HotKeyBinding>) {
    HOTKEY_DRAFT_BINDING.with(|draft| {
        *draft.borrow_mut() = binding;
    });
}

fn reset_hotkey_editor(panel: id) {
    set_hotkey_draft_binding(None);
    set_hotkey_recording(panel, false, None);
    refresh_hotkey_views(panel);
}

fn is_hotkey_recording(panel: id) -> bool {
    unsafe {
        let controller = controller_from_panel(panel);
        if controller == nil {
            return false;
        }

        let controller = &*(controller as *const Object);
        let recording: BOOL = *controller.get_ivar("hotkey_recording");
        recording == YES
    }
}

fn set_hotkey_recording(panel: id, recording: bool, message: Option<&str>) {
    unsafe {
        let controller = controller_from_panel(panel);
        if controller == nil {
            return;
        }

        let controller = &mut *(controller as *mut Object);
        let subtitle_label: id = *controller.get_ivar("settings_subtitle_label");
        let record_button: id = *controller.get_ivar("settings_record_button");
        let save_button: id = *controller.get_ivar("settings_save_button");
        let cancel_button: id = *controller.get_ivar("settings_cancel_button");
        let has_draft = hotkey_draft_binding().is_some();

        controller.set_ivar("hotkey_recording", if recording { YES } else { NO });
        set_label_text(
            subtitle_label,
            message.unwrap_or(if recording {
                HOTKEY_SETTINGS_RECORDING_HINT
            } else if has_draft {
                HOTKEY_SETTINGS_PENDING_HINT
            } else {
                HOTKEY_SETTINGS_IDLE_HINT
            }),
        );
        set_view_hidden(record_button, recording || has_draft);
        set_view_hidden(save_button, !recording && !has_draft);
        set_button_enabled(save_button, has_draft);
        set_view_hidden(cancel_button, !recording && !has_draft);
    }
}

fn cancel_hotkey_recording_if_needed(panel: id) {
    if is_hotkey_recording(panel) || hotkey_draft_binding().is_some() {
        reset_hotkey_editor(panel);
    }
}

fn handle_hotkey_recording_key_event(panel: id, event: id) -> bool {
    unsafe {
        if panel == nil || event == nil || !is_hotkey_recording(panel) {
            return false;
        }

        let is_repeat: BOOL = msg_send![event, isARepeat];
        if is_repeat == YES {
            return true;
        }

        let key_code: u16 = msg_send![event, keyCode];
        if key_code == HOTKEY_CAPTURE_CANCEL_KEYCODE {
            cancel_hotkey_recording_if_needed(panel);
            return true;
        }

        let modifier_flags: usize = msg_send![event, modifierFlags];
        let binding = match hotkey::binding_from_key_event(key_code, modifier_flags as u64) {
            Ok(binding) => binding,
            Err(message) => {
                set_hotkey_recording(panel, true, Some(&message));
                return true;
            }
        };

        if binding == hotkey::current_binding() {
            set_hotkey_draft_binding(None);
            refresh_hotkey_views(panel);
            set_hotkey_recording(panel, false, Some("该组合已是当前绑定，无需重复保存"));
            return true;
        }

        set_hotkey_draft_binding(Some(binding));
        refresh_hotkey_views(panel);
        set_hotkey_recording(panel, false, None);

        true
    }
}

fn refresh_hotkey_views(panel: id) {
    unsafe {
        let controller = controller_from_panel(panel);
        if controller == nil {
            return;
        }

        let controller = &*(controller as *const Object);
        let footer_modifier_label: id = *controller.get_ivar("footer_modifier_label");
        let footer_key_label: id = *controller.get_ivar("footer_key_label");
        let settings_preview_label: id = *controller.get_ivar("settings_preview_label");

        let binding = hotkey::current_binding();
        let preview_binding = hotkey_draft_binding().unwrap_or(binding);
        let preview_text = preview_binding.preview_text();

        set_label_text(footer_modifier_label, binding.modifier_symbol());
        set_label_text(footer_key_label, binding.key_label());
        set_label_text(settings_preview_label, &preview_text);
    }
}

fn pulse_hotkey_labels(panel: id) {
    unsafe {
        let controller = controller_from_panel(panel);
        if controller == nil {
            return;
        }

        let controller = &*(controller as *const Object);
        let footer_modifier_label: id = *controller.get_ivar("footer_modifier_label");
        let footer_key_label: id = *controller.get_ivar("footer_key_label");
        let settings_preview_label: id = *controller.get_ivar("settings_preview_label");

        for view in [
            footer_modifier_label,
            footer_key_label,
            settings_preview_label,
        ] {
            if view == nil {
                continue;
            }

            let _: () = msg_send![view, setAlphaValue: 0.66f64];
            animate_view_alpha(view, 1.0);
        }
    }
}

fn transition_settings_page(panel: id, show_settings: bool) {
    unsafe {
        let controller = controller_from_panel(panel);
        if controller == nil {
            return;
        }

        let main_page = main_page_from_panel(panel);
        let settings_page = settings_page_from_panel(panel);
        if main_page == nil || settings_page == nil {
            return;
        }

        let controller = &mut *(controller as *mut Object);
        if !show_settings {
            reset_hotkey_editor(panel);
        }
        controller.set_ivar("settings_visible", if show_settings { YES } else { NO });

        let main_page_origin = if show_settings {
            NSPoint::new(-22.0, 0.0)
        } else {
            NSPoint::new(0.0, 0.0)
        };
        let settings_page_origin = if show_settings {
            NSPoint::new(0.0, 0.0)
        } else {
            NSPoint::new(SETTINGS_PAGE_OFFSET_X, 0.0)
        };

        if show_settings {
            let _: BOOL = msg_send![panel, makeFirstResponder: nil];
            reset_hotkey_editor(panel);
        }

        animate_view_origin(main_page, main_page_origin, 0.18);
        animate_view_alpha(main_page, if show_settings { 0.18 } else { 1.0 });

        animate_view_origin(settings_page, settings_page_origin, 0.22);
        animate_view_alpha(settings_page, if show_settings { 1.0 } else { 0.0 });

        if show_settings {
            pulse_hotkey_labels(panel);
        } else {
            let input_field = input_field_from_panel(panel);
            if input_field != nil {
                let _: BOOL = msg_send![panel, makeFirstResponder: input_field];
                let _: () = msg_send![input_field, selectText: nil];
            }
        }
    }
}

fn clear_input_field(input_field: id) {
    unsafe {
        let empty = NSString::alloc(nil).init_str("");
        let _: () = msg_send![input_field, setStringValue: empty];
    }
}

fn build_panel(controller: id) -> id {
    unsafe {
        let frame = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(PANEL_WIDTH, PANEL_HEIGHT),
        );
        let style_mask = NS_WINDOW_STYLE_TITLED
            | NS_WINDOW_STYLE_CLOSABLE
            | NS_WINDOW_STYLE_NONACTIVATING_PANEL
            | NS_WINDOW_STYLE_FULL_SIZE_CONTENT_VIEW;

        let panel: id = msg_send![class!(NSPanel), alloc];
        let panel: id = msg_send![
            panel,
            initWithContentRect: frame
            styleMask: style_mask
            backing: NSBackingStoreType::NSBackingStoreBuffered
            defer: NO
        ];

        let clear_color: id = msg_send![class!(NSColor), clearColor];
        let title = NSString::alloc(nil).init_str("");
        let _: () = msg_send![panel, setTitle: title];
        let _: () = msg_send![panel, setOpaque: NO];
        let _: () = msg_send![panel, setBackgroundColor: clear_color];
        let _: () = msg_send![panel, setHasShadow: YES];
        let _: () = msg_send![panel, setReleasedWhenClosed: NO];
        let _: () = msg_send![panel, setBecomesKeyOnlyIfNeeded: YES];
        let _: () = msg_send![panel, setTitleVisibility: NS_WINDOW_TITLE_HIDDEN];
        let _: () = msg_send![panel, setTitlebarAppearsTransparent: YES];
        let _: () = msg_send![panel, setMovableByWindowBackground: YES];
        let _: () = msg_send![panel, setDelegate: controller];

        hide_standard_window_buttons(panel);

        let content_view: id = msg_send![panel, contentView];
        style_view(
            content_view,
            Some((4, 4, 6, 0.98)),
            Some((24, 24, 27, 1.0, 1.0)),
            PANEL_RADIUS,
        );

        let main_page = build_page_container(NSPoint::new(0.0, 0.0));
        add_brand_header(main_page);
        add_search_bar(main_page, controller);
        add_divider(
            main_page,
            NSRect::new(NSPoint::new(0.0, DIVIDER_Y), NSSize::new(PANEL_WIDTH, 1.0)),
            (25, 25, 29, 1.0),
        );
        let history_document_view = add_history_list(main_page);
        let (footer_modifier_label, footer_key_label) = add_footer(main_page);

        let settings_views = add_settings_page(controller);
        let _: () = msg_send![content_view, addSubview: main_page];
        let _: () = msg_send![content_view, addSubview: settings_views.page];
        add_header_actions(content_view, controller);

        let controller = &mut *(controller as *mut Object);
        controller.set_ivar("history_document_view", history_document_view);
        controller.set_ivar("main_page", main_page);
        controller.set_ivar("settings_page", settings_views.page);
        controller.set_ivar("settings_subtitle_label", settings_views.subtitle_label);
        controller.set_ivar("footer_modifier_label", footer_modifier_label);
        controller.set_ivar("footer_key_label", footer_key_label);
        controller.set_ivar("settings_preview_label", settings_views.preview_label);
        controller.set_ivar("settings_record_button", settings_views.record_button);
        controller.set_ivar("settings_save_button", settings_views.save_button);
        controller.set_ivar("settings_cancel_button", settings_views.cancel_button);
        controller.set_ivar("hotkey_recording", NO);
        controller.set_ivar("settings_visible", NO);

        panel
    }
}

fn add_brand_header(content_view: id) {
    unsafe {
        let title_label = build_text_label(
            NSRect::new(
                NSPoint::new(PANEL_SIDE_MARGIN, PANEL_HEIGHT - 50.0),
                NSSize::new(120.0, 20.0),
            ),
            "最近复制",
            13.0,
            false,
            (161, 161, 170, 0.94),
            0,
        );
        let subtitle_label = build_text_label(
            NSRect::new(
                NSPoint::new(PANEL_SIDE_MARGIN, PANEL_HEIGHT - 72.0),
                NSSize::new(180.0, 16.0),
            ),
            "点击条目即可粘贴",
            11.0,
            false,
            (82, 82, 91, 1.0),
            0,
        );

        let _: () = msg_send![content_view, addSubview: title_label];
        let _: () = msg_send![content_view, addSubview: subtitle_label];
    }
}

fn add_header_actions(content_view: id, controller: id) {
    unsafe {
        let right_edge = PANEL_WIDTH - PANEL_SIDE_MARGIN - HEADER_BUTTON_SIZE;
        let top_y = PANEL_HEIGHT - 56.0;

        if let Some(settings_image) = build_svg_image(SETTINGS_ICON_SVG, HEADER_ICON_SIZE) {
            let settings_button = build_icon_button(
                NSRect::new(
                    NSPoint::new(
                        right_edge - (HEADER_BUTTON_SIZE + HEADER_BUTTON_SPACING),
                        top_y,
                    ),
                    NSSize::new(HEADER_BUTTON_SIZE, HEADER_BUTTON_SIZE),
                ),
                controller,
                sel!(showSettings:),
                "设置",
                settings_image,
            );
            let _: () = msg_send![content_view, addSubview: settings_button];
        }

        if let Some(delete_image) = build_svg_image(DELETE_ICON_SVG, HEADER_ICON_SIZE) {
            let delete_button = build_icon_button(
                NSRect::new(
                    NSPoint::new(right_edge, top_y),
                    NSSize::new(HEADER_BUTTON_SIZE, HEADER_BUTTON_SIZE),
                ),
                controller,
                sel!(clearInput:),
                "清空输入",
                delete_image,
            );
            let _: () = msg_send![content_view, addSubview: delete_button];
        }
    }
}

fn add_search_bar(content_view: id, controller: id) {
    unsafe {
        let search_frame = NSRect::new(
            NSPoint::new(PANEL_SIDE_MARGIN, SEARCH_BAR_Y),
            NSSize::new(PANEL_WIDTH - PANEL_SIDE_MARGIN * 2.0, SEARCH_BAR_HEIGHT),
        );
        let search_container: id = msg_send![class!(NSView), alloc];
        let search_container: id = msg_send![search_container, initWithFrame: search_frame];
        style_view(
            search_container,
            Some((7, 7, 8, 1.0)),
            Some((52, 52, 58, 1.0, 1.0)),
            18.0,
        );

        if let Some(search_image) =
            build_colored_svg_image(SEARCH_ICON_SVG, "#A1A1AA", SEARCH_ICON_SIZE)
        {
            let search_icon_view = build_image_view(
                NSRect::new(NSPoint::new(18.0, 17.0), NSSize::new(21.0, 21.0)),
                search_image,
            );
            let _: () = msg_send![search_container, addSubview: search_icon_view];
        }

        let input_frame = NSRect::new(
            NSPoint::new(52.0, 10.0),
            NSSize::new(
                PANEL_WIDTH - PANEL_SIDE_MARGIN * 2.0 - 112.0,
                SEARCH_BAR_HEIGHT - 20.0,
            ),
        );
        let input_field: id = msg_send![class!(NSTextField), alloc];
        let input_field: id = msg_send![input_field, initWithFrame: input_frame];
        let placeholder = NSString::alloc(nil).init_str("搜索");
        let font: id = msg_send![class!(NSFont), systemFontOfSize: 20.0];

        let _: () = msg_send![input_field, setTag: INPUT_TAG];
        let _: () = msg_send![input_field, setPlaceholderString: placeholder];
        let _: () = msg_send![input_field, setTarget: controller];
        let _: () = msg_send![input_field, setAction: sel!(submitText:)];
        let _: () = msg_send![input_field, setBezeled: NO];
        let _: () = msg_send![input_field, setBordered: NO];
        let _: () = msg_send![input_field, setDrawsBackground: NO];
        let _: () = msg_send![input_field, setEditable: YES];
        let _: () = msg_send![input_field, setSelectable: YES];
        let _: () = msg_send![input_field, setFont: font];
        let _: () = msg_send![input_field, setFocusRingType: NS_FOCUS_RING_TYPE_NONE];
        let _: () = msg_send![input_field, setTextColor: ns_color(244, 244, 245, 1.0)];

        let filter_shell: id = msg_send![class!(NSView), alloc];
        let filter_shell: id = msg_send![
            filter_shell,
            initWithFrame: NSRect::new(
                NSPoint::new(
                    PANEL_WIDTH - PANEL_SIDE_MARGIN * 2.0 - SEARCH_RIGHT_BUTTON_SIZE - 12.0,
                    11.0,
                ),
                NSSize::new(SEARCH_RIGHT_BUTTON_SIZE, SEARCH_RIGHT_BUTTON_SIZE),
            )
        ];
        style_view(
            filter_shell,
            Some((10, 10, 12, 1.0)),
            Some((60, 60, 66, 1.0, 1.0)),
            17.0,
        );

        if let Some(filter_image) =
            build_colored_svg_image(FILTER_ICON_SVG, "#A1A1AA", SEARCH_RIGHT_ICON_SIZE)
        {
            let filter_view = build_image_view(
                NSRect::new(NSPoint::new(8.0, 8.0), NSSize::new(18.0, 18.0)),
                filter_image,
            );
            let _: () = msg_send![filter_shell, addSubview: filter_view];
        }

        let _: () = msg_send![search_container, addSubview: input_field];
        let _: () = msg_send![search_container, addSubview: filter_shell];
        let _: () = msg_send![content_view, addSubview: search_container];
    }
}

fn add_footer(content_view: id) -> (id, id) {
    unsafe {
        let binding = hotkey::current_binding();
        let footer_frame = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(PANEL_WIDTH, FOOTER_HEIGHT),
        );
        let footer: id = msg_send![class!(NSView), alloc];
        let footer: id = msg_send![footer, initWithFrame: footer_frame];
        style_view(footer, Some((3, 3, 5, 1.0)), None, 0.0);

        add_divider(
            footer,
            NSRect::new(
                NSPoint::new(0.0, FOOTER_HEIGHT - 1.0),
                NSSize::new(PANEL_WIDTH, 1.0),
            ),
            (24, 24, 28, 1.0),
        );

        let label = build_text_label(
            NSRect::new(NSPoint::new(18.0, 18.0), NSSize::new(96.0, 20.0)),
            "全局快捷键",
            14.0,
            true,
            (229, 231, 235, 1.0),
            0,
        );
        let (option_key, modifier_label) = build_keycap(
            NSRect::new(NSPoint::new(126.0, 10.0), NSSize::new(46.0, 36.0)),
            binding.modifier_symbol(),
        );
        let plus = build_text_label(
            NSRect::new(NSPoint::new(178.0, 16.0), NSSize::new(20.0, 20.0)),
            "+",
            19.0,
            true,
            (229, 231, 235, 1.0),
            1,
        );
        let (letter_key, key_label) = build_keycap(
            NSRect::new(NSPoint::new(206.0, 10.0), NSSize::new(42.0, 36.0)),
            binding.key_label(),
        );

        let _: () = msg_send![footer, addSubview: label];
        let _: () = msg_send![footer, addSubview: option_key];
        let _: () = msg_send![footer, addSubview: plus];
        let _: () = msg_send![footer, addSubview: letter_key];
        let _: () = msg_send![content_view, addSubview: footer];

        (modifier_label, key_label)
    }
}

fn build_keycap(frame: NSRect, text: &str) -> (id, id) {
    unsafe {
        let keycap: id = msg_send![class!(NSView), alloc];
        let keycap: id = msg_send![keycap, initWithFrame: frame];
        style_view(
            keycap,
            Some((41, 41, 46, 1.0)),
            Some((71, 71, 80, 1.0, 1.0)),
            9.0,
        );

        let label = build_text_label(
            NSRect::new(
                NSPoint::new(0.0, 8.0),
                NSSize::new(frame.size.width, frame.size.height - 16.0),
            ),
            text,
            17.0,
            true,
            (244, 244, 245, 1.0),
            1,
        );
        let _: () = msg_send![keycap, addSubview: label];
        (keycap, label)
    }
}

fn build_page_container(origin: NSPoint) -> id {
    unsafe {
        let page: id = msg_send![class!(NSView), alloc];
        let page: id = msg_send![
            page,
            initWithFrame: NSRect::new(origin, NSSize::new(PANEL_WIDTH, PANEL_HEIGHT))
        ];
        style_view(page, Some((4, 4, 6, 0.0)), None, 0.0);
        page
    }
}

struct SettingsPageViews {
    page: id,
    subtitle_label: id,
    preview_label: id,
    record_button: id,
    save_button: id,
    cancel_button: id,
}

fn add_settings_page(controller: id) -> SettingsPageViews {
    unsafe {
        let binding = hotkey::current_binding();
        let settings_page = build_page_container(NSPoint::new(SETTINGS_PAGE_OFFSET_X, 0.0));
        let _: () = msg_send![settings_page, setAlphaValue: 0.0f64];

        let title = build_text_label(
            NSRect::new(
                NSPoint::new(PANEL_SIDE_MARGIN, SETTINGS_TITLE_Y),
                NSSize::new(220.0, 28.0),
            ),
            "快捷键设置",
            24.0,
            true,
            (244, 244, 245, 0.97),
            0,
        );
        let subtitle = build_text_label(
            NSRect::new(
                NSPoint::new(PANEL_SIDE_MARGIN, SETTINGS_SUBTITLE_Y),
                NSSize::new(PANEL_WIDTH - PANEL_SIDE_MARGIN * 2.0, 18.0),
            ),
            HOTKEY_SETTINGS_IDLE_HINT,
            13.0,
            false,
            (113, 113, 122, 1.0),
            0,
        );

        let preview_shell: id = msg_send![class!(NSView), alloc];
        let preview_shell: id = msg_send![
            preview_shell,
            initWithFrame: NSRect::new(
                NSPoint::new(PANEL_SIDE_MARGIN, SETTINGS_PREVIEW_Y),
                NSSize::new(SETTINGS_CARD_WIDTH, SETTINGS_PREVIEW_HEIGHT),
            )
        ];
        style_view(
            preview_shell,
            Some((8, 10, 14, 1.0)),
            Some((34, 50, 78, 1.0, 1.0)),
            20.0,
        );

        let preview_caption = build_text_label(
            NSRect::new(NSPoint::new(20.0, 114.0), NSSize::new(120.0, 18.0)),
            "绑定预览",
            12.0,
            false,
            (113, 113, 122, 1.0),
            0,
        );
        let preview_label = build_text_label(
            NSRect::new(NSPoint::new(20.0, 58.0), NSSize::new(320.0, 36.0)),
            &binding.preview_text(),
            26.0,
            true,
            (248, 250, 252, 1.0),
            0,
        );
        let preview_hint = build_text_label(
            NSRect::new(NSPoint::new(20.0, 24.0), NSSize::new(320.0, 16.0)),
            "仅支持一个修饰键 + 字母 / 数字",
            11.0,
            false,
            (82, 82, 91, 1.0),
            0,
        );

        let record_button = build_action_button(
            NSRect::new(
                NSPoint::new(PANEL_SIDE_MARGIN, SETTINGS_ACTION_BUTTON_Y),
                NSSize::new(SETTINGS_CARD_WIDTH, SETTINGS_ACTION_BUTTON_HEIGHT),
            ),
            controller,
            sel!(beginHotkeyRecording:),
            "录制新快捷键",
            (12, 70, 173, 1.0),
            (34, 111, 244, 1.0, 1.0),
            (248, 250, 252, 1.0),
        );
        let save_button = build_action_button(
            NSRect::new(
                NSPoint::new(
                    PANEL_SIDE_MARGIN + SETTINGS_ACTION_BUTTON_WIDTH + SETTINGS_ACTION_BUTTON_GAP,
                    SETTINGS_ACTION_BUTTON_Y,
                ),
                NSSize::new(SETTINGS_ACTION_BUTTON_WIDTH, SETTINGS_ACTION_BUTTON_HEIGHT),
            ),
            controller,
            sel!(saveHotkeyBinding:),
            "保存",
            (12, 70, 173, 1.0),
            (34, 111, 244, 1.0, 1.0),
            (248, 250, 252, 1.0),
        );
        let cancel_button = build_action_button(
            NSRect::new(
                NSPoint::new(PANEL_SIDE_MARGIN, SETTINGS_ACTION_BUTTON_Y),
                NSSize::new(SETTINGS_ACTION_BUTTON_WIDTH, SETTINGS_ACTION_BUTTON_HEIGHT),
            ),
            controller,
            sel!(cancelHotkeyRecording:),
            "取消",
            (25, 25, 30, 1.0),
            (63, 63, 70, 1.0, 1.0),
            (244, 244, 245, 1.0),
        );
        let _: () = msg_send![save_button, setHidden: YES];
        let _: () = msg_send![cancel_button, setHidden: YES];

        let _: () = msg_send![preview_shell, addSubview: preview_caption];
        let _: () = msg_send![preview_shell, addSubview: preview_label];
        let _: () = msg_send![preview_shell, addSubview: preview_hint];

        let _: () = msg_send![settings_page, addSubview: title];
        let _: () = msg_send![settings_page, addSubview: subtitle];
        let _: () = msg_send![settings_page, addSubview: preview_shell];
        let _: () = msg_send![settings_page, addSubview: record_button];
        let _: () = msg_send![settings_page, addSubview: save_button];
        let _: () = msg_send![settings_page, addSubview: cancel_button];

        SettingsPageViews {
            page: settings_page,
            subtitle_label: subtitle,
            preview_label,
            record_button,
            save_button,
            cancel_button,
        }
    }
}

fn build_action_button(
    frame: NSRect,
    controller: id,
    action: Sel,
    title: &str,
    background: (u8, u8, u8, f64),
    border: (u8, u8, u8, f64, f64),
    text_color: (u8, u8, u8, f64),
) -> id {
    unsafe {
        let button_class = register_history_row_button_class();
        let button: id = msg_send![button_class, alloc];
        let button: id = msg_send![button, initWithFrame: frame];
        let empty_title = NSString::alloc(nil).init_str("");
        let _: () = msg_send![button, setTitle: empty_title];
        let _: () = msg_send![button, setBordered: NO];
        let _: () = msg_send![button, setTarget: controller];
        let _: () = msg_send![button, setAction: action];
        let _: () = msg_send![button, setFocusRingType: NS_FOCUS_RING_TYPE_NONE];
        style_view(button, Some(background), Some(border), 16.0);

        let bounds = NSRect::new(NSPoint::new(0.0, 0.0), frame.size);
        let hover_overlay: id = msg_send![class!(NSView), alloc];
        let hover_overlay: id = msg_send![hover_overlay, initWithFrame: bounds];
        style_view(hover_overlay, Some((255, 255, 255, 0.08)), None, 16.0);
        let _: () = msg_send![hover_overlay, setAlphaValue: 0.0f64];

        let tracking_area: id = msg_send![class!(NSTrackingArea), alloc];
        let tracking_area: id = msg_send![
            tracking_area,
            initWithRect: bounds
            options: NS_TRACKING_MOUSE_ENTERED_AND_EXITED | NS_TRACKING_ACTIVE_ALWAYS | NS_TRACKING_IN_VISIBLE_RECT
            owner: button
            userInfo: nil
        ];
        let _: () = msg_send![button, addTrackingArea: tracking_area];
        let button_mut = &mut *(button as *mut Object);
        button_mut.set_ivar("hover_overlay", hover_overlay);

        let title_label = build_text_label(
            NSRect::new(
                NSPoint::new(0.0, 11.0),
                NSSize::new(frame.size.width, frame.size.height - 22.0),
            ),
            title,
            14.0,
            true,
            text_color,
            1,
        );

        let _: () = msg_send![button, addSubview: hover_overlay];
        let _: () = msg_send![button, addSubview: title_label];

        button
    }
}

fn hide_standard_window_buttons(panel: id) {
    unsafe {
        for button_kind in [
            NS_WINDOW_BUTTON_CLOSE,
            NS_WINDOW_BUTTON_MINIMIZE,
            NS_WINDOW_BUTTON_ZOOM,
        ] {
            let button: id = msg_send![panel, standardWindowButton: button_kind];
            if button != nil {
                let _: () = msg_send![button, setHidden: YES];
            }
        }
    }
}

fn add_history_list(content_view: id) -> id {
    unsafe {
        let scroll_frame = NSRect::new(
            NSPoint::new(0.0, HISTORY_SCROLL_Y),
            NSSize::new(PANEL_WIDTH, HISTORY_SCROLL_HEIGHT),
        );

        let scroll_view: id = msg_send![class!(NSScrollView), alloc];
        let scroll_view: id = msg_send![scroll_view, initWithFrame: scroll_frame];
        let _: () = msg_send![scroll_view, setHasVerticalScroller: YES];
        let _: () = msg_send![scroll_view, setAutohidesScrollers: YES];
        let _: () = msg_send![scroll_view, setDrawsBackground: NO];
        let _: () = msg_send![scroll_view, setBorderType: 0usize];

        let document_frame = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(PANEL_WIDTH, HISTORY_SCROLL_HEIGHT),
        );
        let document_view: id = msg_send![class!(NSView), alloc];
        let document_view: id = msg_send![document_view, initWithFrame: document_frame];
        style_view(document_view, Some((4, 4, 6, 0.0)), None, 0.0);

        let _: () = msg_send![scroll_view, setDocumentView: document_view];
        let _: () = msg_send![content_view, addSubview: scroll_view];

        document_view
    }
}

fn render_history_entries(panel: id) {
    unsafe {
        let document_view = history_document_view_from_panel(panel);
        if document_view == nil {
            return;
        }

        remove_all_subviews(document_view);

        let entries = match history::recent_entries(HISTORY_LIMIT) {
            Ok(entries) => entries,
            Err(err) => {
                error!("加载历史条目失败: {err}");
                add_history_placeholder(document_view, "历史加载失败");
                return;
            }
        };

        let total_height = f64::max(
            HISTORY_SCROLL_HEIGHT,
            entries.len() as f64 * HISTORY_ROW_HEIGHT,
        );
        let _: () = msg_send![
            document_view,
            setFrame: NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(PANEL_WIDTH, total_height),
            )
        ];

        if entries.is_empty() {
            add_history_placeholder(document_view, "复制的内容会显示在这里");
            return;
        }

        let controller: id = msg_send![panel, delegate];
        for (index, entry) in entries.iter().enumerate() {
            let origin_y = total_height - HISTORY_ROW_HEIGHT * (index as f64 + 1.0);
            add_history_row(
                document_view,
                controller,
                entry,
                origin_y,
                PANEL_WIDTH,
                index,
            );
        }
    }
}

fn remove_all_subviews(view: id) {
    unsafe {
        let subviews: id = msg_send![view, subviews];
        if subviews == nil {
            return;
        }

        while nsarray_count(subviews) > 0 {
            let index = nsarray_count(subviews) - 1;
            let subview = nsarray_object(subviews, index);
            let _: () = msg_send![subview, removeFromSuperview];
        }
    }
}

fn add_history_placeholder(document_view: id, text: &str) {
    unsafe {
        let label = build_text_label(
            NSRect::new(
                NSPoint::new(42.0, HISTORY_SCROLL_HEIGHT * 0.5 - 16.0),
                NSSize::new(PANEL_WIDTH - 84.0, 32.0),
            ),
            text,
            15.0,
            false,
            (113, 113, 122, 1.0),
            1,
        );
        let _: () = msg_send![document_view, addSubview: label];
    }
}

fn add_history_row(
    document_view: id,
    controller: id,
    entry: &HistoryEntry,
    origin_y: f64,
    width: f64,
    index: usize,
) {
    unsafe {
        let row_frame = NSRect::new(
            NSPoint::new(0.0, origin_y),
            NSSize::new(width, HISTORY_ROW_HEIGHT),
        );
        let row_bounds = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(width, HISTORY_ROW_HEIGHT),
        );

        let row_view: id = msg_send![class!(NSView), alloc];
        let row_view: id = msg_send![row_view, initWithFrame: row_frame];

        let hover_overlay: id = msg_send![class!(NSView), alloc];
        let hover_overlay: id = msg_send![hover_overlay, initWithFrame: row_bounds];
        style_view(hover_overlay, Some((18, 20, 27, 1.0)), None, 0.0);
        let _: () = msg_send![hover_overlay, setAlphaValue: 0.0f64];

        let icon_box: id = msg_send![class!(NSView), alloc];
        let icon_box: id = msg_send![
            icon_box,
            initWithFrame: NSRect::new(
                NSPoint::new(18.0, 23.0),
                NSSize::new(HISTORY_ICON_BOX_SIZE, HISTORY_ICON_BOX_SIZE),
            )
        ];
        style_view(icon_box, Some((5, 23, 52, 1.0)), None, 11.0);

        if let Some(icon_image) =
            build_colored_svg_image(LIST_ICON_SVG, "#1785FF", HISTORY_ICON_SIZE)
        {
            let icon_view = build_image_view(
                NSRect::new(NSPoint::new(11.0, 11.0), NSSize::new(18.0, 18.0)),
                icon_image,
            );
            let _: () = msg_send![icon_box, addSubview: icon_view];
        }

        let title_label = build_text_label(
            NSRect::new(
                NSPoint::new(80.0, HISTORY_ROW_HEIGHT - 42.0),
                NSSize::new(width - 150.0, 24.0),
            ),
            &entry.title,
            16.0,
            true,
            (250, 250, 250, 1.0),
            0,
        );
        let subtitle_label = build_text_label(
            NSRect::new(NSPoint::new(80.0, 16.0), NSSize::new(width - 150.0, 20.0)),
            &entry.subtitle,
            13.0,
            false,
            (161, 161, 170, 1.0),
            0,
        );

        let shortcut_text = if index < 9 {
            format!("⌘ {}", index + 1)
        } else {
            String::new()
        };
        let shortcut_label = build_text_label(
            NSRect::new(
                NSPoint::new(width - 56.0, HISTORY_ROW_HEIGHT - 46.0),
                NSSize::new(34.0, 18.0),
            ),
            &shortcut_text,
            13.0,
            false,
            (113, 113, 122, 1.0),
            2,
        );

        let separator = build_divider_view(
            NSRect::new(NSPoint::new(80.0, 0.0), NSSize::new(width - 98.0, 1.0)),
            (22, 22, 26, 1.0),
        );

        let action_button_class = register_history_row_button_class();
        let action_button: id = msg_send![action_button_class, alloc];
        let action_button: id = msg_send![action_button, initWithFrame: row_bounds];
        let empty_title = NSString::alloc(nil).init_str("");
        let tooltip = NSString::alloc(nil).init_str(&entry.title);
        let _: () = msg_send![action_button, setTitle: empty_title];
        let _: () = msg_send![action_button, setBordered: NO];
        let _: () = msg_send![action_button, setTarget: controller];
        let _: () = msg_send![action_button, setAction: sel!(historyEntrySelected:)];
        let _: () = msg_send![action_button, setTag: entry.id as isize];
        let _: () = msg_send![action_button, setToolTip: tooltip];
        let _: () = msg_send![action_button, setFocusRingType: NS_FOCUS_RING_TYPE_NONE];

        let tracking_area: id = msg_send![class!(NSTrackingArea), alloc];
        let tracking_area: id = msg_send![
            tracking_area,
            initWithRect: row_bounds
            options: NS_TRACKING_MOUSE_ENTERED_AND_EXITED | NS_TRACKING_ACTIVE_ALWAYS | NS_TRACKING_IN_VISIBLE_RECT
            owner: action_button
            userInfo: nil
        ];
        let _: () = msg_send![action_button, addTrackingArea: tracking_area];
        let action_button_mut = &mut *(action_button as *mut Object);
        action_button_mut.set_ivar("hover_overlay", hover_overlay);

        let _: () = msg_send![row_view, addSubview: hover_overlay];
        let _: () = msg_send![row_view, addSubview: icon_box];
        let _: () = msg_send![row_view, addSubview: title_label];
        let _: () = msg_send![row_view, addSubview: subtitle_label];
        let _: () = msg_send![row_view, addSubview: shortcut_label];
        let _: () = msg_send![row_view, addSubview: separator];
        let _: () = msg_send![row_view, addSubview: action_button];
        let _: () = msg_send![document_view, addSubview: row_view];
    }
}

extern "C" fn history_row_mouse_entered(this: &Object, _: Sel, _: id) {
    unsafe {
        let hover_overlay: id = *this.get_ivar("hover_overlay");
        animate_view_alpha(hover_overlay, 1.0);
    }
}

extern "C" fn history_row_mouse_exited(this: &Object, _: Sel, _: id) {
    unsafe {
        let hover_overlay: id = *this.get_ivar("hover_overlay");
        animate_view_alpha(hover_overlay, 0.0);
    }
}

fn nsarray_count(array: id) -> usize {
    unsafe { msg_send![array, count] }
}

fn nsarray_object(array: id, index: usize) -> id {
    unsafe { msg_send![array, objectAtIndex: index] }
}

fn build_icon_button(
    frame: NSRect,
    controller: id,
    action: Sel,
    tooltip_text: &str,
    image: id,
) -> id {
    unsafe {
        let button: id = msg_send![class!(NSButton), alloc];
        let button: id = msg_send![button, initWithFrame: frame];
        let title = NSString::alloc(nil).init_str("");
        let tooltip = NSString::alloc(nil).init_str(tooltip_text);

        let _: () = msg_send![button, setTitle: title];
        let _: () = msg_send![button, setBordered: NO];
        let _: () = msg_send![button, setTarget: controller];
        let _: () = msg_send![button, setAction: action];
        let _: () = msg_send![button, setToolTip: tooltip];
        let _: () = msg_send![button, setImage: image];

        button
    }
}

fn build_image_view(frame: NSRect, image: id) -> id {
    unsafe {
        let image_view: id = msg_send![class!(NSImageView), alloc];
        let image_view: id = msg_send![image_view, initWithFrame: frame];
        let _: () = msg_send![image_view, setImage: image];
        image_view
    }
}

fn build_text_label(
    frame: NSRect,
    text: &str,
    font_size: f64,
    bold: bool,
    color: (u8, u8, u8, f64),
    alignment: usize,
) -> id {
    unsafe {
        let label: id = msg_send![class!(NSTextField), alloc];
        let label: id = msg_send![label, initWithFrame: frame];
        let text_value = NSString::alloc(nil).init_str(text);
        let font: id = if bold {
            msg_send![class!(NSFont), boldSystemFontOfSize: font_size]
        } else {
            msg_send![class!(NSFont), systemFontOfSize: font_size]
        };

        let _: () = msg_send![label, setStringValue: text_value];
        let _: () = msg_send![label, setBezeled: NO];
        let _: () = msg_send![label, setDrawsBackground: NO];
        let _: () = msg_send![label, setEditable: NO];
        let _: () = msg_send![label, setSelectable: NO];
        let _: () = msg_send![label, setFont: font];
        let _: () = msg_send![label, setAlignment: alignment];
        let _: () = msg_send![label, setTextColor: ns_color(color.0, color.1, color.2, color.3)];

        label
    }
}

fn build_divider_view(frame: NSRect, color: (u8, u8, u8, f64)) -> id {
    unsafe {
        let view: id = msg_send![class!(NSView), alloc];
        let view: id = msg_send![view, initWithFrame: frame];
        style_view(view, Some(color), None, 0.0);
        view
    }
}

fn add_divider(parent: id, frame: NSRect, color: (u8, u8, u8, f64)) {
    unsafe {
        let divider = build_divider_view(frame, color);
        let _: () = msg_send![parent, addSubview: divider];
    }
}

fn style_view(
    view: id,
    background: Option<(u8, u8, u8, f64)>,
    border: Option<(u8, u8, u8, f64, f64)>,
    corner_radius: f64,
) {
    unsafe {
        let _: () = msg_send![view, setWantsLayer: YES];
        let layer: id = msg_send![view, layer];
        if layer == nil {
            return;
        }

        let _: () = msg_send![layer, setCornerRadius: corner_radius];
        let _: () = msg_send![layer, setMasksToBounds: YES];

        if let Some((red, green, blue, alpha)) = background {
            let cg_color: id = msg_send![ns_color(red, green, blue, alpha), CGColor];
            let _: () = msg_send![layer, setBackgroundColor: cg_color];
        }

        if let Some((red, green, blue, alpha, width)) = border {
            let cg_color: id = msg_send![ns_color(red, green, blue, alpha), CGColor];
            let _: () = msg_send![layer, setBorderColor: cg_color];
            let _: () = msg_send![layer, setBorderWidth: width];
        }
    }
}

fn animate_view_alpha(view: id, alpha: f64) {
    unsafe {
        if view == nil {
            return;
        }

        let _: () = msg_send![class!(NSAnimationContext), beginGrouping];
        let context: id = msg_send![class!(NSAnimationContext), currentContext];
        let _: () = msg_send![context, setDuration: 0.16f64];
        let animator: id = msg_send![view, animator];
        let _: () = msg_send![animator, setAlphaValue: alpha];
        let _: () = msg_send![class!(NSAnimationContext), endGrouping];
    }
}

fn animate_view_origin(view: id, origin: NSPoint, duration: f64) {
    unsafe {
        if view == nil {
            return;
        }

        let _: () = msg_send![class!(NSAnimationContext), beginGrouping];
        let context: id = msg_send![class!(NSAnimationContext), currentContext];
        let _: () = msg_send![context, setDuration: duration];
        let animator: id = msg_send![view, animator];
        let _: () = msg_send![animator, setFrameOrigin: origin];
        let _: () = msg_send![class!(NSAnimationContext), endGrouping];
    }
}

fn ns_color(red: u8, green: u8, blue: u8, alpha: f64) -> id {
    unsafe {
        msg_send![
            class!(NSColor),
            colorWithCalibratedRed: red as f64 / 255.0
            green: green as f64 / 255.0
            blue: blue as f64 / 255.0
            alpha: alpha
        ]
    }
}

fn build_svg_image(svg: &str, icon_size: u32) -> Option<id> {
    let png_bytes = match icons::render_svg_png(svg, icon_size) {
        Ok(png_bytes) => png_bytes,
        Err(error) => {
            error!("构建 SVG 图标失败: {error}");
            return None;
        }
    };

    build_png_image(&png_bytes, icon_size)
}

fn build_colored_svg_image(svg: &str, color: &str, icon_size: u32) -> Option<id> {
    let colored_svg = svg.replace("currentColor", color);
    build_svg_image(&colored_svg, icon_size)
}

fn build_png_image(png_bytes: &[u8], icon_size: u32) -> Option<id> {
    unsafe {
        let data: id = msg_send![
            class!(NSData),
            dataWithBytes: png_bytes.as_ptr()
            length: png_bytes.len()
        ];
        if data == nil {
            return None;
        }

        let image: id = msg_send![class!(NSImage), alloc];
        let image: id = msg_send![image, initWithData: data];
        if image == nil {
            return None;
        }

        let _: () = msg_send![image, setTemplate: NO];
        let _: () = msg_send![image, setSize: NSSize::new(icon_size as f64, icon_size as f64)];

        Some(image)
    }
}
