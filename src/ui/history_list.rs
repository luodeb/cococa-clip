use cocoa::base::{id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
use log::error;
use objc::{class, msg_send, sel};

use crate::history;
use crate::history::HistoryEntry;
use crate::ui::widgets;

const ROW_HEIGHT: f64 = 72.0;
const HISTORY_LIMIT: usize = 80;

pub fn render(document_view: id, controller: id) {
    unsafe {
        if document_view == nil {
            return;
        }

        clear_subviews(document_view);

        let entries = match history::recent_entries(HISTORY_LIMIT) {
            Ok(entries) => entries,
            Err(err) => {
                error!("加载历史条目失败: {err}");
                add_placeholder(document_view, "历史加载失败");
                return;
            }
        };

        if entries.is_empty() {
            add_placeholder(document_view, "当前还没有历史记录");
            return;
        }

        let frame: NSRect = msg_send![document_view, frame];
        let total_height = f64::max(frame.size.height, entries.len() as f64 * ROW_HEIGHT);
        let _: () = msg_send![
            document_view,
            setFrame: NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(frame.size.width, total_height),
            )
        ];

        for (index, entry) in entries.iter().enumerate() {
            let y = total_height - (index as f64 + 1.0) * ROW_HEIGHT;
            add_row(document_view, controller, entry, y, frame.size.width);
        }
    }
}

fn add_row(document_view: id, controller: id, entry: &HistoryEntry, y: f64, width: f64) {
    unsafe {
        let row: id = msg_send![class!(NSView), alloc];
        let row: id = msg_send![
            row,
            initWithFrame: NSRect::new(NSPoint::new(0.0, y), NSSize::new(width, ROW_HEIGHT))
        ];
        widgets::style_view(row, Some((13, 13, 18, 1.0)), None, 0.0);

        let title = widgets::build_text_label(
            NSRect::new(
                NSPoint::new(16.0, ROW_HEIGHT - 34.0),
                NSSize::new(width - 32.0, 20.0),
            ),
            &entry.title,
            14.0,
            true,
            (243, 244, 246, 1.0),
            0,
        );
        let subtitle = widgets::build_text_label(
            NSRect::new(NSPoint::new(16.0, 14.0), NSSize::new(width - 32.0, 18.0)),
            &entry.subtitle,
            12.0,
            false,
            (148, 163, 184, 1.0),
            0,
        );
        let divider = widgets::build_divider(
            NSRect::new(NSPoint::new(16.0, 0.0), NSSize::new(width - 32.0, 1.0)),
            (31, 41, 55, 1.0),
        );

        let action: id = msg_send![class!(NSButton), alloc];
        let action: id = msg_send![
            action,
            initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, ROW_HEIGHT))
        ];
        let empty_title = NSString::alloc(nil).init_str("");
        let tooltip = NSString::alloc(nil).init_str(&entry.title);
        let _: () = msg_send![action, setTitle: empty_title];
        let _: () = msg_send![action, setBordered: 0u8];
        let _: () = msg_send![action, setTarget: controller];
        let _: () = msg_send![action, setAction: sel!(historyRowPressed:)];
        let _: () = msg_send![action, setTag: entry.id as isize];
        let _: () = msg_send![action, setToolTip: tooltip];

        let _: () = msg_send![row, addSubview: title];
        let _: () = msg_send![row, addSubview: subtitle];
        let _: () = msg_send![row, addSubview: divider];
        let _: () = msg_send![row, addSubview: action];
        let _: () = msg_send![document_view, addSubview: row];
    }
}

fn add_placeholder(document_view: id, text: &str) {
    unsafe {
        let label = widgets::build_text_label(
            NSRect::new(NSPoint::new(24.0, 120.0), NSSize::new(320.0, 24.0)),
            text,
            14.0,
            false,
            (148, 163, 184, 1.0),
            0,
        );
        let _: () = msg_send![document_view, addSubview: label];
    }
}

fn clear_subviews(view: id) {
    unsafe {
        let subviews: id = msg_send![view, subviews];
        if subviews == nil {
            return;
        }

        loop {
            let count: usize = msg_send![subviews, count];
            if count == 0 {
                break;
            }
            let subview: id = msg_send![subviews, objectAtIndex: count - 1];
            let _: () = msg_send![subview, removeFromSuperview];
        }
    }
}
