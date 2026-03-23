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

const HISTORY_SCROLL_TOP: f64 = 118.0;
const HISTORY_SCROLL_BOTTOM: f64 = 54.0;

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
            Some((10, 10, 13, 0.98)),
            Some((40, 40, 45, 1.0, 1.0)),
            18.0,
        );

        let (title_bar, _, _) = widgets::build_window_title_bar(
            "剪贴板历史",
            "回车粘贴输入内容，点击历史条目直接粘贴",
            width,
        );
        let _: () = msg_send![title_bar, setFrameOrigin: NSPoint::new(0.0, height - 64.0)];

        let settings_button = widgets::build_button(
            NSRect::new(NSPoint::new(width - 96.0, height - 48.0), NSSize::new(76.0, 30.0)),
            "设置",
            controller,
            sel!(openSettings:),
            (24, 52, 91, 1.0),
            (36, 88, 160, 1.0, 1.0),
            (243, 244, 246, 1.0),
        );

        let _: () = msg_send![content, addSubview: title_bar];
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
                NSPoint::new(16.0, height - 112.0),
                NSSize::new(width - 32.0, 46.0),
            )
        ];
        widgets::style_view(shell, Some((18, 18, 23, 1.0)), Some((52, 52, 62, 1.0, 1.0)), 12.0);

        let input_field: id = msg_send![class!(NSTextField), alloc];
        let input_field: id = msg_send![
            input_field,
            initWithFrame: NSRect::new(NSPoint::new(14.0, 8.0), NSSize::new(width - 60.0, 30.0))
        ];

        let placeholder = NSString::alloc(nil).init_str("输入后按回车直接粘贴");
        let font: id = msg_send![class!(NSFont), systemFontOfSize: 15.0];
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
        let _: () = msg_send![input_field, setTextColor: widgets::ns_color(230, 230, 235, 1.0)];

        let _: () = msg_send![shell, addSubview: input_field];
        let _: () = msg_send![content_view, addSubview: shell];
        input_field
    }
}

pub fn build_history_section(content_view: id, width: f64, height: f64) -> id {
    unsafe {
        let scroll_height = height - HISTORY_SCROLL_TOP - HISTORY_SCROLL_BOTTOM;
        let scroll_view: id = msg_send![class!(NSScrollView), alloc];
        let scroll_view: id = msg_send![
            scroll_view,
            initWithFrame: NSRect::new(
                NSPoint::new(0.0, HISTORY_SCROLL_BOTTOM),
                NSSize::new(width, scroll_height),
            )
        ];

        let _: () = msg_send![scroll_view, setHasVerticalScroller: YES];
        let _: () = msg_send![scroll_view, setAutohidesScrollers: YES];
        let _: () = msg_send![scroll_view, setDrawsBackground: NO];

        let document_view: id = msg_send![class!(NSView), alloc];
        let document_view: id = msg_send![
            document_view,
            initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, scroll_height))
        ];
        widgets::style_view(document_view, Some((10, 10, 13, 0.0)), None, 0.0);

        let _: () = msg_send![scroll_view, setDocumentView: document_view];
        let _: () = msg_send![content_view, addSubview: scroll_view];

        document_view
    }
}

pub fn build_footer(content_view: id, width: f64) -> id {
    unsafe {
        let footer: id = msg_send![class!(NSView), alloc];
        let footer: id = msg_send![
            footer,
            initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, 54.0))
        ];
        widgets::style_view(footer, Some((16, 16, 20, 1.0)), None, 0.0);

        let divider = widgets::build_divider(
            NSRect::new(NSPoint::new(0.0, 53.0), NSSize::new(width, 1.0)),
            (34, 34, 39, 1.0),
        );
        let tip = widgets::build_text_label(
            NSRect::new(NSPoint::new(16.0, 18.0), NSSize::new(120.0, 18.0)),
            "当前快捷键",
            13.0,
            false,
            (148, 163, 184, 1.0),
            0,
        );
        let value = widgets::build_text_label(
            NSRect::new(NSPoint::new(128.0, 15.0), NSSize::new(200.0, 22.0)),
            "-",
            15.0,
            true,
            (244, 244, 245, 1.0),
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
