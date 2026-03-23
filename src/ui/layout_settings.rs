use cocoa::appkit::NSBackingStoreType;
use cocoa::base::{NO, YES, id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
use objc::{class, msg_send, sel};

use crate::ui::widgets;

const NS_WINDOW_STYLE_TITLED: usize = 1 << 0;
const NS_WINDOW_STYLE_CLOSABLE: usize = 1 << 1;
const NS_WINDOW_STYLE_NONACTIVATING_PANEL: usize = 1 << 7;
const NS_WINDOW_STYLE_FULL_SIZE_CONTENT_VIEW: usize = 1 << 15;

const NS_WINDOW_BUTTON_CLOSE: usize = 0;
const NS_WINDOW_BUTTON_MINIMIZE: usize = 1;
const NS_WINDOW_BUTTON_ZOOM: usize = 2;
const NS_WINDOW_TITLE_HIDDEN: usize = 1;
const NS_FOCUS_RING_TYPE_NONE: usize = 1;

pub fn build_settings_window(
    controller: id,
    main_window: id,
    width: f64,
    height: f64,
) -> id {
    unsafe {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, height));
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
        let title = NSString::alloc(nil).init_str("设置");
        let _: () = msg_send![panel, setTitle: title];
        let _: () = msg_send![panel, setOpaque: NO];
        let _: () = msg_send![panel, setBackgroundColor: clear_color];
        let _: () = msg_send![panel, setHasShadow: YES];
        let _: () = msg_send![panel, setReleasedWhenClosed: NO];
        let _: () = msg_send![panel, setTitleVisibility: NS_WINDOW_TITLE_HIDDEN];
        let _: () = msg_send![panel, setTitlebarAppearsTransparent: YES];
        let _: () = msg_send![panel, setMovableByWindowBackground: YES];
        let _: () = msg_send![panel, setDelegate: controller];

        hide_standard_window_buttons(panel);
        place_settings_window(main_window, panel, width, height);

        let content: id = msg_send![panel, contentView];
        widgets::style_view_with_shadow(
            content,
            Some((8, 15, 30, 0.98)),
            Some((41, 69, 110, 0.95, 1.0)),
            20.0,
            (0, 0, 0, 0.35, 18.0, 0.0, -3.0),
        );

        let (title_bar, _, _) = widgets::build_window_title_bar("快捷键设置", "录制新组合后保存即可生效", width);
        let _: () = msg_send![title_bar, setFrameOrigin: NSPoint::new(0.0, height - 82.0)];
        let close_button = widgets::build_button(
            NSRect::new(NSPoint::new(width - 114.0, height - 56.0), NSSize::new(92.0, 34.0)),
            "关闭",
            controller,
            sel!(closeSettings:),
            (63, 24, 31, 1.0),
            (146, 40, 52, 1.0, 1.0),
            (255, 228, 232, 1.0),
        );
        let _: () = msg_send![close_button, setFocusRingType: NS_FOCUS_RING_TYPE_NONE];
        let _: () = msg_send![close_button, setRefusesFirstResponder: YES];

        let _: () = msg_send![content, addSubview: title_bar];
        let _: () = msg_send![content, addSubview: close_button];

        panel
    }
}

pub fn build_settings_form(
    content_view: id,
    controller: id,
    width: f64,
    height: f64,
) -> (id, id, id, id, id) {
    unsafe {
        let hint = widgets::build_text_label(
            NSRect::new(
                NSPoint::new(24.0, height - 112.0),
                NSSize::new(width - 48.0, 20.0),
            ),
            "点击“开始录制”后，按下一个修饰键 + 主键",
            12.5,
            false,
            (125, 164, 210, 1.0),
            0,
        );

        let preview_box: id = msg_send![class!(NSView), alloc];
        let preview_box: id = msg_send![
            preview_box,
            initWithFrame: NSRect::new(
                NSPoint::new(24.0, height - 206.0),
                NSSize::new(width - 48.0, 88.0),
            )
        ];
        widgets::style_view(
            preview_box,
            Some((15, 28, 48, 0.97)),
            Some((55, 103, 169, 0.9, 1.0)),
            14.0,
        );

        let preview_title = widgets::build_text_label(
            NSRect::new(NSPoint::new(14.0, 58.0), NSSize::new(200.0, 16.0)),
            "新的快捷键预览",
            11.5,
            false,
            (129, 171, 220, 1.0),
            0,
        );
        let preview_value = widgets::build_text_label(
            NSRect::new(NSPoint::new(14.0, 18.0), NSSize::new(280.0, 34.0)),
            "-",
            24.0,
            true,
            (230, 245, 255, 1.0),
            0,
        );

        let record_button = widgets::build_button(
            NSRect::new(NSPoint::new(24.0, 44.0), NSSize::new(width - 48.0, 42.0)),
            "开始录制",
            controller,
            sel!(beginRecord:),
            (26, 80, 132, 1.0),
            (58, 146, 228, 1.0, 1.0),
            (232, 245, 255, 1.0),
        );

        let save_button = widgets::build_button(
            NSRect::new(NSPoint::new(24.0, 44.0), NSSize::new((width - 60.0) / 2.0, 42.0)),
            "保存",
            controller,
            sel!(saveRecord:),
            (16, 98, 65, 1.0),
            (39, 196, 128, 1.0, 1.0),
            (221, 255, 236, 1.0),
        );

        let cancel_button = widgets::build_button(
            NSRect::new(
                NSPoint::new(36.0 + (width - 60.0) / 2.0, 44.0),
                NSSize::new((width - 60.0) / 2.0, 42.0),
            ),
            "取消",
            controller,
            sel!(cancelRecord:),
            (65, 24, 28, 1.0),
            (153, 44, 54, 1.0, 1.0),
            (255, 225, 231, 1.0),
        );

        let _: () = msg_send![record_button, setFocusRingType: NS_FOCUS_RING_TYPE_NONE];
        let _: () = msg_send![save_button, setFocusRingType: NS_FOCUS_RING_TYPE_NONE];
        let _: () = msg_send![cancel_button, setFocusRingType: NS_FOCUS_RING_TYPE_NONE];
        let _: () = msg_send![record_button, setRefusesFirstResponder: YES];
        let _: () = msg_send![save_button, setRefusesFirstResponder: YES];
        let _: () = msg_send![cancel_button, setRefusesFirstResponder: YES];

        let _: () = msg_send![save_button, setHidden: YES];
        let _: () = msg_send![cancel_button, setHidden: YES];

        let _: () = msg_send![preview_box, addSubview: preview_title];
        let _: () = msg_send![preview_box, addSubview: preview_value];

        let _: () = msg_send![content_view, addSubview: hint];
        let _: () = msg_send![content_view, addSubview: preview_box];
        let _: () = msg_send![content_view, addSubview: record_button];
        let _: () = msg_send![content_view, addSubview: save_button];
        let _: () = msg_send![content_view, addSubview: cancel_button];

        (hint, preview_value, record_button, save_button, cancel_button)
    }
}

pub fn place_settings_window(main_window: id, settings_window: id, width: f64, height: f64) {
    unsafe {
        let main_frame: NSRect = msg_send![main_window, frame];
        let origin = NSPoint::new(
            main_frame.origin.x + (main_frame.size.width - width) * 0.5,
            main_frame.origin.y + (main_frame.size.height - height) * 0.5,
        );
        let _: () = msg_send![settings_window, setFrameOrigin: origin];
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
