use cocoa::appkit::NSBackingStoreType;
use cocoa::base::{BOOL, NO, YES, id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
use log::{debug, error};
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel};
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
const CHECK_ICON_SVG: &str = include_str!("../assets/icons/check-circle.svg");
const DELETE_ICON_SVG: &str = include_str!("../assets/icons/trash.svg");
const SEARCH_ICON_SVG: &str = include_str!("../assets/icons/search.svg");
const FILTER_ICON_SVG: &str = include_str!("../assets/icons/filter.svg");
const LIST_ICON_SVG: &str = include_str!("../assets/icons/list.svg");

pub fn register_controller_class() -> *const Class {
    static ONCE: Once = Once::new();
    static mut CLASS: *const Class = std::ptr::null();

    ONCE.call_once(|| unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("ClipPanelController", superclass)
            .expect("ClipPanelController class declaration failed");
        decl.add_ivar::<id>("panel");
        decl.add_ivar::<id>("history_document_view");

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

extern "C" fn show_settings(_: &Object, _: Sel, _: id) {
    debug!("settings toolbar button clicked");
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

        let input_field = input_field_from_panel(panel);
        if input_field != nil {
            let _: BOOL = msg_send![panel, makeFirstResponder: input_field];
            let _: () = msg_send![input_field, selectText: nil];
        }
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
        let controller: id = msg_send![panel, delegate];
        if controller == nil {
            return nil;
        }

        let controller = &*(controller as *const Object);
        *controller.get_ivar("history_document_view")
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

        add_brand_header(content_view);
        add_header_actions(content_view, controller);
        add_search_bar(content_view, controller);
        add_divider(
            content_view,
            NSRect::new(NSPoint::new(0.0, DIVIDER_Y), NSSize::new(PANEL_WIDTH, 1.0)),
            (25, 25, 29, 1.0),
        );

        let history_document_view = add_history_list(content_view);
        let controller = &mut *(controller as *mut Object);
        controller.set_ivar("history_document_view", history_document_view);

        add_footer(content_view);

        panel
    }
}

fn add_brand_header(content_view: id) {
    let _ = content_view;
}

fn add_header_actions(content_view: id, controller: id) {
    unsafe {
        let right_edge = PANEL_WIDTH - PANEL_SIDE_MARGIN - HEADER_BUTTON_SIZE;
        let top_y = PANEL_HEIGHT - 56.0;

        if let Some(settings_image) = build_svg_image(SETTINGS_ICON_SVG, HEADER_ICON_SIZE) {
            let settings_button = build_icon_button(
                NSRect::new(
                    NSPoint::new(
                        right_edge - (HEADER_BUTTON_SIZE + HEADER_BUTTON_SPACING) * 2.0,
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

        if let Some(check_image) =
            build_colored_svg_image(CHECK_ICON_SVG, "#A1A1AA", HEADER_ICON_SIZE)
        {
            let check_button = build_icon_button(
                NSRect::new(
                    NSPoint::new(
                        right_edge - (HEADER_BUTTON_SIZE + HEADER_BUTTON_SPACING),
                        top_y,
                    ),
                    NSSize::new(HEADER_BUTTON_SIZE, HEADER_BUTTON_SIZE),
                ),
                controller,
                sel!(submitText:),
                "执行粘贴",
                check_image,
            );
            let _: () = msg_send![content_view, addSubview: check_button];
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

fn add_footer(content_view: id) {
    unsafe {
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
        let option_key = build_keycap(
            NSRect::new(NSPoint::new(126.0, 10.0), NSSize::new(46.0, 36.0)),
            "⌥",
        );
        let plus = build_text_label(
            NSRect::new(NSPoint::new(178.0, 16.0), NSSize::new(20.0, 20.0)),
            "+",
            19.0,
            true,
            (229, 231, 235, 1.0),
            1,
        );
        let letter_key = build_keycap(
            NSRect::new(NSPoint::new(206.0, 10.0), NSSize::new(42.0, 36.0)),
            "C",
        );

        let _: () = msg_send![footer, addSubview: label];
        let _: () = msg_send![footer, addSubview: option_key];
        let _: () = msg_send![footer, addSubview: plus];
        let _: () = msg_send![footer, addSubview: letter_key];
        let _: () = msg_send![content_view, addSubview: footer];
    }
}

fn build_keycap(frame: NSRect, text: &str) -> id {
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
        keycap
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
