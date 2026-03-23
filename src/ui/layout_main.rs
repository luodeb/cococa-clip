use cocoa::appkit::NSBackingStoreType;
use cocoa::base::{BOOL, NO, YES, id, nil};
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

const HEADER_HEIGHT: f64 = 34.0;
const INPUT_AREA_HEIGHT: f64 = 52.0;
const FOOTER_HEIGHT: f64 = 48.0;
const CONTENT_SIDE_PADDING: f64 = 14.0;

pub const INPUT_TAG: isize = 3001;

pub fn build_main_window(controller: id, width: f64, height: f64) -> id {
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
        let title = NSString::alloc(nil).init_str("Cococa Clip");
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

        let content: id = msg_send![panel, contentView];
        widgets::style_view(
            content,
            Some((12, 14, 20, 0.98)),
            Some((45, 50, 62, 0.95, 1.0)),
            16.0,
        );

        let settings_button = widgets::build_button(
            NSRect::new(NSPoint::new(width - 88.0, height - 30.0), NSSize::new(70.0, 20.0)),
            "设置",
            controller,
            sel!(openSettings:),
            (45, 50, 62, 1.0),
            (72, 78, 95, 1.0, 1.0),
            (214, 220, 233, 1.0),
        );

        let clear_button = widgets::build_button(
            NSRect::new(NSPoint::new(width - 168.0, height - 30.0), NSSize::new(74.0, 20.0)),
            "清空历史",
            controller,
            sel!(clearHistory:),
            (60, 33, 36, 1.0),
            (92, 51, 57, 1.0, 1.0),
            (234, 213, 216, 1.0),
        );

        let _: () = msg_send![content, addSubview: clear_button];
        let _: () = msg_send![content, addSubview: settings_button];

        panel
    }
}

pub fn build_input_section(content_view: id, controller: id, width: f64, height: f64) -> id {
    unsafe {
        let shell: id = msg_send![class!(NSView), alloc];
        let shell: id = msg_send![
            shell,
            initWithFrame: NSRect::new(
                NSPoint::new(CONTENT_SIDE_PADDING, height - HEADER_HEIGHT - INPUT_AREA_HEIGHT - 10.0),
                NSSize::new(width - CONTENT_SIDE_PADDING * 2.0, INPUT_AREA_HEIGHT),
            )
        ];
        widgets::style_view(shell, Some((19, 22, 30, 1.0)), Some((56, 60, 74, 1.0, 1.0)), 10.0);

        let search_icon = widgets::build_text_label(
            NSRect::new(NSPoint::new(12.0, 14.0), NSSize::new(20.0, 20.0)),
            "⌕",
            16.0,
            false,
            (138, 146, 170, 1.0),
            0,
        );

        let input_field: id = msg_send![class!(NSTextField), alloc];
        let input_field: id = msg_send![
            input_field,
            initWithFrame: NSRect::new(
                NSPoint::new(36.0, 10.0),
                NSSize::new(width - CONTENT_SIDE_PADDING * 2.0 - 48.0, 32.0)
            )
        ];

        let placeholder = NSString::alloc(nil).init_str("搜索历史内容（文本、文件、图片）");
        let font: id = msg_send![class!(NSFont), systemFontOfSize: 14.0];
        let _: () = msg_send![input_field, setTag: INPUT_TAG];
        let _: () = msg_send![input_field, setPlaceholderString: placeholder];
        let _: () = msg_send![input_field, setTarget: controller];
        let _: () = msg_send![input_field, setAction: sel!(searchChanged:)];
        let _: () = msg_send![input_field, setDelegate: controller];
        let _: () = msg_send![input_field, setFont: font];
        let _: () = msg_send![input_field, setFocusRingType: NS_FOCUS_RING_TYPE_NONE];
        let _: () = msg_send![input_field, setBezeled: NO];
        let _: () = msg_send![input_field, setBordered: NO];
        let _: () = msg_send![input_field, setDrawsBackground: NO];
        let _: () = msg_send![input_field, setTextColor: widgets::ns_color(218, 223, 236, 1.0)];

        let _: () = msg_send![shell, addSubview: search_icon];
        let _: () = msg_send![shell, addSubview: input_field];
        let _: () = msg_send![content_view, addSubview: shell];
        input_field
    }
}

pub fn build_history_section(content_view: id, width: f64, height: f64) -> id {
    unsafe {
        let history_top = height - HEADER_HEIGHT - INPUT_AREA_HEIGHT - 12.0;
        let history_bottom = FOOTER_HEIGHT + 8.0;
        let history_height = history_top - history_bottom;

        let card: id = msg_send![class!(NSView), alloc];
        let card: id = msg_send![
            card,
            initWithFrame: NSRect::new(
                NSPoint::new(CONTENT_SIDE_PADDING, history_bottom),
                NSSize::new(width - CONTENT_SIDE_PADDING * 2.0, history_height),
            )
        ];
        widgets::style_view(card, Some((16, 18, 24, 1.0)), Some((56, 60, 74, 1.0, 1.0)), 12.0);

        let scroll_height = history_height - 16.0;
        let scroll_view: id = msg_send![class!(NSScrollView), alloc];
        let scroll_view: id = msg_send![
            scroll_view,
            initWithFrame: NSRect::new(
                NSPoint::new(8.0, 8.0),
                NSSize::new(width - CONTENT_SIDE_PADDING * 2.0 - 16.0, scroll_height),
            )
        ];

        let _: () = msg_send![scroll_view, setHasVerticalScroller: YES];
        let _: () = msg_send![scroll_view, setAutohidesScrollers: YES];
        let _: () = msg_send![scroll_view, setDrawsBackground: NO];

        let document_view: id = msg_send![class!(NSView), alloc];
        let document_view: id = msg_send![
            document_view,
            initWithFrame: NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(width - CONTENT_SIDE_PADDING * 2.0 - 16.0, scroll_height),
            )
        ];
        widgets::style_view(document_view, Some((16, 18, 24, 0.01)), None, 0.0);

        let _: () = msg_send![scroll_view, setDocumentView: document_view];
        let _: () = msg_send![card, addSubview: scroll_view];
        let _: () = msg_send![content_view, addSubview: card];

        document_view
    }
}

pub fn build_footer(content_view: id, width: f64) -> id {
    unsafe {
        let footer: id = msg_send![class!(NSView), alloc];
        let footer: id = msg_send![
            footer,
            initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, FOOTER_HEIGHT))
        ];
        widgets::style_view(footer, Some((12, 14, 20, 0.98)), Some((45, 50, 62, 1.0, 1.0)), 0.0);

        let divider = widgets::build_divider(
            NSRect::new(NSPoint::new(0.0, FOOTER_HEIGHT - 1.0), NSSize::new(width, 1.0)),
            (52, 58, 74, 1.0),
        );
        let tip = widgets::build_text_label(
            NSRect::new(NSPoint::new(14.0, 15.0), NSSize::new(100.0, 18.0)),
            "快捷键",
            11.0,
            false,
            (138, 146, 170, 1.0),
            0,
        );
        let value = widgets::build_text_label(
            NSRect::new(NSPoint::new(72.0, 12.0), NSSize::new(160.0, 22.0)),
            "-",
            14.0,
            true,
            (210, 216, 231, 1.0),
            0,
        );

        let _: () = msg_send![footer, addSubview: divider];
        let _: () = msg_send![footer, addSubview: tip];
        let _: () = msg_send![footer, addSubview: value];
        let _: () = msg_send![content_view, addSubview: footer];

        value
    }
}

pub fn locate_input_field(main_window: id) -> id {
    unsafe {
        let content_view: id = msg_send![main_window, contentView];
        msg_send![content_view, viewWithTag: INPUT_TAG]
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

pub fn is_window_visible(window: id) -> bool {
    unsafe {
        if window == nil {
            return false;
        }
        let visible: BOOL = msg_send![window, isVisible];
        visible == YES
    }
}
