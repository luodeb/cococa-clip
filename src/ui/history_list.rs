use cocoa::base::{id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
use log::{error, warn};
use objc::runtime::Object;
use objc::{class, msg_send, sel};

use crate::history;
use crate::history::HistoryEntry;
use crate::ui::controller_state;
use crate::ui::widgets;

const HISTORY_LIMIT: usize = 80;
const ROW_GAP: f64 = 6.0;
const ROW_SIDE_PADDING: f64 = 10.0;
const ROW_VERTICAL_PADDING: f64 = 10.0;
const TEXT_LINE_HEIGHT: f64 = 18.0;
const MAX_TEXT_LINES: usize = 3;
const TEXT_CONTENT_FONT_SIZE: f64 = 13.5;
const IMAGE_MIN_HEIGHT: f64 = 80.0;
const IMAGE_MAX_HEIGHT: f64 = 220.0;

struct RenderRow {
    id: i64,
    title: String,
    display_lines: Vec<String>,
    image_size: Option<(f64, f64)>,
}

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

        let query = current_search_query(controller);
        let normalized_query = query.trim().to_lowercase();
        let filtered_entries: Vec<&HistoryEntry> = if normalized_query.is_empty() {
            entries.iter().collect()
        } else {
            entries
                .iter()
                .filter(|entry| {
                    entry.title.to_lowercase().contains(&normalized_query)
                        || entry.subtitle.to_lowercase().contains(&normalized_query)
                })
                .collect()
        };

        if filtered_entries.is_empty() {
            if normalized_query.is_empty() {
                add_placeholder(document_view, "当前还没有历史记录");
            } else {
                add_placeholder(document_view, "没有匹配的历史内容");
            }
            return;
        }

        let frame: NSRect = msg_send![document_view, frame];
        let available_width = frame.size.width;

        let text_width = available_width - (ROW_SIDE_PADDING + 4.0) * 2.0;

        let mut render_rows = Vec::with_capacity(filtered_entries.len());
        let mut row_heights = Vec::with_capacity(filtered_entries.len());
        let mut total_content_height = 0.0;

        for entry in &filtered_entries {
            let display_text = match history::display_text_for_entry(entry.id) {
                Ok(Some(text)) if !text.trim().is_empty() => text,
                _ => entry.title.clone(),
            };
            let image_size = preferred_image_size(entry.id, available_width);
            let display_lines = if image_size.is_none() {
                wrapped_text_lines(&display_text, text_width, MAX_TEXT_LINES)
            } else {
                Vec::new()
            };
            let row_height = estimate_row_height(image_size, &display_lines);

            render_rows.push(RenderRow {
                id: entry.id,
                title: entry.title.clone(),
                display_lines,
                image_size,
            });
            row_heights.push(row_height);
            total_content_height += row_height;
        }

        if filtered_entries.len() > 1 {
            total_content_height += ROW_GAP * (filtered_entries.len() as f64 - 1.0);
        }

        let total_height = f64::max(frame.size.height, total_content_height);
        let _: () = msg_send![
            document_view,
            setFrame: NSRect::new(
                NSPoint::new(0.0, 0.0),
                NSSize::new(frame.size.width, total_height),
            )
        ];

        let mut y_cursor = total_height;
        for (index, row_data) in render_rows.iter().enumerate() {
            let row_height = row_heights[index];
            y_cursor -= row_height;
            add_row(document_view, controller, row_data, y_cursor, available_width, row_height);
            if index + 1 < render_rows.len() {
                y_cursor -= ROW_GAP;
            }
        }
    }
}

fn estimate_row_height(image_size: Option<(f64, f64)>, display_lines: &[String]) -> f64 {
    if let Some((_, image_height)) = image_size {
        return image_height + ROW_VERTICAL_PADDING * 2.0;
    }

    let line_count = display_lines.len().clamp(1, MAX_TEXT_LINES);
    let raw_height = ROW_VERTICAL_PADDING * 2.0 + line_count as f64 * TEXT_LINE_HEIGHT;
    f64::max(54.0, raw_height)
}

fn add_row(document_view: id, controller: id, row_data: &RenderRow, y: f64, width: f64, row_height: f64) {
    unsafe {
        let row: id = msg_send![class!(NSView), alloc];
        let row: id = msg_send![
            row,
            initWithFrame: NSRect::new(NSPoint::new(0.0, y), NSSize::new(width, row_height))
        ];
        widgets::style_view(row, Some((19, 22, 30, 1.0)), Some((56, 60, 74, 0.8, 1.0)), 10.0);

        let mut has_image_preview = false;

        if let Some((image_width, image_height)) = row_data.image_size {
            if let Ok(Some(image_bytes)) = history::preview_image_data(row_data.id) {
                let data: id = msg_send![
                    class!(NSData),
                    dataWithBytes: image_bytes.as_ptr()
                    length: image_bytes.len()
                ];
                if data == nil {
                    warn!(
                        "image preview decode failed: entry_id={}, reason=NSData is nil, bytes={}",
                        row_data.id,
                        image_bytes.len()
                    );
                } else {
                    let image: id = msg_send![class!(NSImage), alloc];
                    let image: id = msg_send![image, initWithData: data];
                    if image == nil {
                        warn!(
                            "image preview decode failed: entry_id={}, reason=NSImage initWithData returned nil, bytes={}",
                            row_data.id,
                            image_bytes.len()
                        );
                    } else {
                        let image_view: id = msg_send![class!(NSImageView), alloc];
                        let image_view: id = msg_send![
                            image_view,
                            initWithFrame: NSRect::new(
                                NSPoint::new((width - image_width) * 0.5, (row_height - image_height) * 0.5),
                                NSSize::new(image_width, image_height),
                            )
                        ];
                        let _: () = msg_send![image_view, setImage: image];
                        let _: () = msg_send![image_view, setImageScaling: 3usize];
                        widgets::style_view(image_view, Some((24, 27, 35, 1.0)), Some((72, 78, 95, 1.0, 1.0)), 6.0);
                        let _: () = msg_send![row, addSubview: image_view];
                        has_image_preview = true;
                    }
                }
            }
        }

        if !has_image_preview {
            let lines = &row_data.display_lines;
            let line_count = lines.len().clamp(1, MAX_TEXT_LINES);
            let text_start_y = row_height - ROW_VERTICAL_PADDING - TEXT_LINE_HEIGHT;
            let text_width = width - (ROW_SIDE_PADDING + 4.0) * 2.0;

            for (index, line) in lines.iter().take(line_count).enumerate() {
                let label = widgets::build_text_label(
                    NSRect::new(
                        NSPoint::new(ROW_SIDE_PADDING + 4.0, text_start_y - index as f64 * TEXT_LINE_HEIGHT),
                        NSSize::new(text_width, TEXT_LINE_HEIGHT),
                    ),
                    line,
                    TEXT_CONTENT_FONT_SIZE,
                    index == 0,
                    if index == 0 {
                        (218, 223, 236, 1.0)
                    } else {
                        (136, 146, 168, 1.0)
                    },
                    0,
                );
                let _: () = msg_send![row, addSubview: label];
            }
        }

        let action: id = msg_send![class!(NSButton), alloc];
        let action: id = msg_send![
            action,
            initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, row_height))
        ];
        let empty_title = NSString::alloc(nil).init_str("");
        let tooltip = NSString::alloc(nil).init_str(&row_data.title);
        let _: () = msg_send![action, setTitle: empty_title];
        let _: () = msg_send![action, setBordered: 0u8];
        let _: () = msg_send![action, setTarget: controller];
        let _: () = msg_send![action, setAction: sel!(historyRowPressed:)];
        let _: () = msg_send![action, setTag: row_data.id as isize];
        let _: () = msg_send![action, setToolTip: tooltip];

        let _: () = msg_send![row, addSubview: action];
        let _: () = msg_send![document_view, addSubview: row];
    }
}

fn preferred_image_size(entry_id: i64, row_width: f64) -> Option<(f64, f64)> {
    unsafe {
        let image_bytes = history::preview_image_data(entry_id).ok()??;
        let data: id = msg_send![
            class!(NSData),
            dataWithBytes: image_bytes.as_ptr()
            length: image_bytes.len()
        ];
        if data == nil {
            return None;
        }

        let image: id = msg_send![class!(NSImage), alloc];
        let image: id = msg_send![image, initWithData: data];
        if image == nil {
            return None;
        }

        let size: NSSize = msg_send![image, size];
        if size.width <= 0.0 || size.height <= 0.0 {
            return None;
        }

        let max_width = row_width - ROW_SIDE_PADDING * 2.0;
        let max_height = IMAGE_MAX_HEIGHT;
        let scale = f64::min(max_width / size.width, max_height / size.height);
        let scale = f64::min(scale, 1.0);

        let mut draw_width = size.width * scale;
        let mut draw_height = size.height * scale;

        if draw_height < IMAGE_MIN_HEIGHT {
            let min_scale = IMAGE_MIN_HEIGHT / size.height;
            draw_width = f64::min(size.width * min_scale, max_width);
            draw_height = f64::min(size.height * min_scale, IMAGE_MAX_HEIGHT);
        }

        Some((draw_width, draw_height))
    }
}

fn wrapped_text_lines(text: &str, text_width: f64, max_lines: usize) -> Vec<String> {
    let max_units = estimate_units_per_line(text_width);
    if max_units == 0 || max_lines == 0 {
        return vec![text.trim().to_owned()];
    }

    let mut lines = Vec::new();
    let mut truncated = false;

    for raw_line in text.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut current = String::new();
        let mut current_units = 0usize;

        for ch in trimmed.chars() {
            let units = char_display_units(ch);
            if !current.is_empty() && current_units + units > max_units {
                lines.push(current.clone());
                if lines.len() >= max_lines {
                    truncated = true;
                    break;
                }
                current.clear();
                current_units = 0;
            }

            current.push(ch);
            current_units += units;
        }

        if truncated {
            break;
        }

        if !current.is_empty() && lines.len() < max_lines {
            lines.push(current);
        } else if !current.is_empty() {
            truncated = true;
        }

        if lines.len() >= max_lines {
            if raw_line.chars().count() > lines.last().map(|s| s.chars().count()).unwrap_or(0) {
                truncated = true;
            }
            break;
        }
    }

    if lines.is_empty() {
        lines.push(text.trim().to_owned());
    }

    if lines.len() > max_lines {
        lines.truncate(max_lines);
        truncated = true;
    }

    if truncated {
        append_ascii_ellipsis(&mut lines, max_units);
    }

    lines
}

fn estimate_units_per_line(text_width: f64) -> usize {
    if text_width <= 0.0 {
        return 0;
    }

    // Roughly map rendered width to character units. ASCII ~=1 unit, CJK ~=2 units.
    let unit_px = TEXT_CONTENT_FONT_SIZE * 0.68;
    let units = (text_width / unit_px).floor() as usize;
    units.max(8)
}

fn append_ascii_ellipsis(lines: &mut [String], max_units: usize) {
    if lines.is_empty() || max_units == 0 {
        return;
    }

    let suffix = "...";
    let suffix_units = suffix.chars().map(char_display_units).sum::<usize>();
    let last = match lines.last_mut() {
        Some(line) => line,
        None => return,
    };

    while !last.is_empty() && display_units(last) + suffix_units > max_units {
        last.pop();
    }

    if display_units(last) + suffix_units <= max_units {
        last.push_str(suffix);
    }
}

fn display_units(text: &str) -> usize {
    text.chars().map(char_display_units).sum()
}

fn char_display_units(ch: char) -> usize {
    if ch.is_ascii() {
        1
    } else {
        2
    }
}

fn add_placeholder(document_view: id, text: &str) {
    unsafe {
        let label = widgets::build_text_label(
            NSRect::new(NSPoint::new(18.0, 120.0), NSSize::new(300.0, 24.0)),
            text,
            13.0,
            false,
            (138, 146, 170, 1.0),
            0,
        );
        let _: () = msg_send![document_view, addSubview: label];
    }
}

fn current_search_query(controller: id) -> String {
    unsafe {
        if controller == nil {
            return String::new();
        }

        let controller = &*(controller as *const Object);
        let input = controller_state::input_field(controller);
        if input == nil {
            return String::new();
        }

        let value: id = msg_send![input, stringValue];
        if value == nil {
            return String::new();
        }

        let ptr: *const std::os::raw::c_char = msg_send![value, UTF8String];
        if ptr.is_null() {
            return String::new();
        }

        std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

fn clear_subviews(view: id) {
    unsafe {
        loop {
            let subviews: id = msg_send![view, subviews];
            if subviews == nil {
                break;
            }

            let count: usize = msg_send![subviews, count];
            if count == 0 {
                break;
            }

            let subview: id = msg_send![subviews, objectAtIndex: count - 1];
            let _: () = msg_send![subview, removeFromSuperview];
        }
    }
}
