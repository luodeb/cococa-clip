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

const TITLE_BAR_HEIGHT: f64 = 82.0;
const CONTENT_PADDING: f64 = 24.0;
const SECTION_GAP: f64 = 12.0;

const PREVIEW_CARD_HEIGHT: f64 = 88.0;
const HINT_HEIGHT: f64 = 20.0;
const ACTION_ROW_HEIGHT: f64 = 42.0;
const AUTOSTART_CARD_HEIGHT: f64 = 64.0;

struct VerticalFlow {
    x: f64,
    width: f64,
    cursor: f64,
    gap: f64,
}

impl VerticalFlow {
    fn new(window_width: f64, window_height: f64) -> Self {
        Self {
            x: CONTENT_PADDING,
            width: window_width - CONTENT_PADDING * 2.0,
            cursor: window_height - TITLE_BAR_HEIGHT - CONTENT_PADDING,
            gap: SECTION_GAP,
        }
    }

    fn place(&mut self, height: f64) -> NSRect {
        let y = self.cursor - height;
        self.cursor = y - self.gap;
        NSRect::new(NSPoint::new(self.x, y), NSSize::new(self.width, height))
    }
}

pub fn build_settings_window(controller: id, _main_window: id, width: f64, height: f64) -> id {
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

        let content: id = msg_send![panel, contentView];
        widgets::style_view(
            content,
            Some((12, 14, 20, 0.98)),
            Some((45, 50, 62, 0.95, 1.0)),
            16.0,
        );

        let title_bar: id = msg_send![class!(NSView), alloc];
        let title_bar: id = msg_send![
            title_bar,
            initWithFrame: NSRect::new(
                NSPoint::new(0.0, height - TITLE_BAR_HEIGHT),
                NSSize::new(width, TITLE_BAR_HEIGHT)
            )
        ];
        widgets::style_view(
            title_bar,
            Some((16, 18, 24, 0.98)),
            Some((52, 58, 74, 1.0, 1.0)),
            0.0,
        );

        let title_label = widgets::build_text_label(
            NSRect::new(
                NSPoint::new(CONTENT_PADDING, TITLE_BAR_HEIGHT - 42.0),
                NSSize::new(width - CONTENT_PADDING * 2.0 - 92.0, 24.0),
            ),
            "快捷键与启动设置",
            18.0,
            true,
            (224, 228, 238, 1.0),
            0,
        );

        let subtitle_label = widgets::build_text_label(
            NSRect::new(
                NSPoint::new(CONTENT_PADDING, TITLE_BAR_HEIGHT - 64.0),
                NSSize::new(width - CONTENT_PADDING * 2.0 - 92.0, 18.0),
            ),
            "优先修改快捷键，其次调整开机自启",
            12.0,
            false,
            (138, 146, 170, 1.0),
            0,
        );

        let close_button = widgets::build_button(
            NSRect::new(
                NSPoint::new(width - CONTENT_PADDING - 64.0, TITLE_BAR_HEIGHT - 52.0),
                NSSize::new(40.0, 28.0),
            ),
            "关闭",
            controller,
            sel!(closeSettings:),
            (45, 50, 62, 1.0),
            (72, 78, 95, 1.0, 1.0),
            (214, 220, 233, 1.0),
        );

        let divider = widgets::build_divider(
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, 1.0)),
            (52, 58, 74, 1.0),
        );

        let _: () = msg_send![title_bar, addSubview: title_label];
        let _: () = msg_send![title_bar, addSubview: subtitle_label];
        let _: () = msg_send![title_bar, addSubview: close_button];
        let _: () = msg_send![title_bar, addSubview: divider];
        let _: () = msg_send![content, addSubview: title_bar];

        panel
    }
}

#[allow(clippy::type_complexity)]
pub fn build_settings_form(
    content_view: id,
    controller: id,
    width: f64,
    height: f64,
) -> (id, id, id, id, id, id) {
    unsafe {
        let mut flow = VerticalFlow::new(width, height);

        let preview_card_frame = flow.place(PREVIEW_CARD_HEIGHT);
        let preview_card: id = msg_send![class!(NSView), alloc];
        let preview_card: id = msg_send![preview_card, initWithFrame: preview_card_frame];
        widgets::style_view(
            preview_card,
            Some((19, 22, 30, 1.0)),
            Some((56, 60, 74, 1.0, 1.0)),
            12.0,
        );

        let preview_tip = widgets::build_text_label(
            NSRect::new(
                NSPoint::new(16.0, PREVIEW_CARD_HEIGHT - 30.0),
                NSSize::new(preview_card_frame.size.width - 32.0, 16.0),
            ),
            "快捷键预览",
            11.0,
            false,
            (138, 146, 170, 1.0),
            0,
        );

        let preview_label = widgets::build_text_label(
            NSRect::new(
                NSPoint::new(16.0, 18.0),
                NSSize::new(preview_card_frame.size.width - 32.0, 42.0),
            ),
            "-",
            24.0,
            true,
            (214, 220, 233, 1.0),
            1,
        );

        let _: () = msg_send![preview_card, addSubview: preview_tip];
        let _: () = msg_send![preview_card, addSubview: preview_label];
        let _: () = msg_send![content_view, addSubview: preview_card];

        let hint_frame = flow.place(HINT_HEIGHT);
        let hint_label = widgets::build_text_label(
            hint_frame,
            "准备好后点击“开始录制”",
            12.0,
            false,
            (145, 156, 184, 1.0),
            0,
        );
        let _: () = msg_send![content_view, addSubview: hint_label];

        let action_frame = flow.place(ACTION_ROW_HEIGHT);
        let action_half = (action_frame.size.width - 12.0) / 2.0;

        let record_button = widgets::build_button(
            NSRect::new(
                NSPoint::new(action_frame.origin.x + (action_frame.size.width - 156.0) / 2.0, action_frame.origin.y),
                NSSize::new(156.0, ACTION_ROW_HEIGHT),
            ),
            "开始录制",
            controller,
            sel!(beginRecord:),
            (56, 63, 79, 1.0),
            (82, 91, 113, 1.0, 1.0),
            (224, 229, 239, 1.0),
        );

        let save_button = widgets::build_button(
            NSRect::new(
                NSPoint::new(action_frame.origin.x, action_frame.origin.y),
                NSSize::new(action_half, ACTION_ROW_HEIGHT),
            ),
            "保存",
            controller,
            sel!(saveRecord:),
            (36, 78, 66, 1.0),
            (58, 112, 95, 1.0, 1.0),
            (220, 241, 235, 1.0),
        );

        let cancel_button = widgets::build_button(
            NSRect::new(
                NSPoint::new(action_frame.origin.x + action_half + 12.0, action_frame.origin.y),
                NSSize::new(action_half, ACTION_ROW_HEIGHT),
            ),
            "取消",
            controller,
            sel!(cancelRecord:),
            (66, 40, 46, 1.0),
            (100, 60, 67, 1.0, 1.0),
            (240, 224, 228, 1.0),
        );

        let _: () = msg_send![save_button, setHidden: YES];
        let _: () = msg_send![cancel_button, setHidden: YES];

        let _: () = msg_send![content_view, addSubview: record_button];
        let _: () = msg_send![content_view, addSubview: save_button];
        let _: () = msg_send![content_view, addSubview: cancel_button];

        let autostart_frame = flow.place(AUTOSTART_CARD_HEIGHT);
        let autostart_card: id = msg_send![class!(NSView), alloc];
        let autostart_card: id = msg_send![autostart_card, initWithFrame: autostart_frame];
        widgets::style_view(
            autostart_card,
            Some((19, 22, 30, 1.0)),
            Some((56, 60, 74, 1.0, 1.0)),
            10.0,
        );

        let autostart_title = widgets::build_text_label(
            NSRect::new(
                NSPoint::new(14.0, AUTOSTART_CARD_HEIGHT - 28.0),
                NSSize::new(220.0, 18.0),
            ),
            "开机自动启动",
            14.0,
            true,
            (220, 225, 237, 1.0),
            0,
        );

        let autostart_desc = widgets::build_text_label(
            NSRect::new(
                NSPoint::new(14.0, 12.0),
                NSSize::new(250.0, 16.0),
            ),
            "登录系统后自动在后台可用",
            11.0,
            false,
            (138, 146, 170, 1.0),
            0,
        );

        let toggle = widgets::build_switch(
            NSRect::new(
                NSPoint::new(autostart_frame.size.width - 64.0, (AUTOSTART_CARD_HEIGHT - 26.0) / 2.0),
                NSSize::new(52.0, 26.0),
            ),
            controller,
            sel!(toggleLaunchAtLogin:),
        );

        let _: () = msg_send![autostart_card, addSubview: autostart_title];
        let _: () = msg_send![autostart_card, addSubview: autostart_desc];
        let _: () = msg_send![autostart_card, addSubview: toggle];
        let _: () = msg_send![content_view, addSubview: autostart_card];

        (
            hint_label,
            preview_label,
            record_button,
            save_button,
            cancel_button,
            toggle,
        )
    }
}

pub fn place_settings_window(main_window: id, settings_window: id, width: f64, height: f64) {
    unsafe {
        if main_window == nil || settings_window == nil {
            return;
        }

        let main_frame: NSRect = msg_send![main_window, frame];

        let mut target_x = main_frame.origin.x + main_frame.size.width + 12.0;
        let mut target_y = main_frame.origin.y + main_frame.size.height - height;

        let main_screen: id = msg_send![main_window, screen];
        let screen: id = if main_screen != nil {
            main_screen
        } else {
            msg_send![class!(NSScreen), mainScreen]
        };

        if screen != nil {
            let visible: NSRect = msg_send![screen, visibleFrame];
            let min_x = visible.origin.x;
            let max_x = visible.origin.x + visible.size.width;
            let min_y = visible.origin.y;
            let max_y = visible.origin.y + visible.size.height;

            if target_x + width > max_x {
                target_x = main_frame.origin.x - width - 12.0;
            }
            if target_x < min_x {
                target_x = min_x + 8.0;
            }

            if target_y + height > max_y {
                target_y = max_y - height;
            }
            if target_y < min_y {
                target_y = min_y;
            }
        }

        let _: () = msg_send![settings_window, setFrameOrigin: NSPoint::new(target_x, target_y)];
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
