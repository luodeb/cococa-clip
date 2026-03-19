use cocoa::base::{BOOL, id, nil};
use cocoa::foundation::NSString;
use log::debug;
use rusqlite::{Connection, params};
use std::cell::RefCell;
use std::ffi::CStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::slice;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::paste;

const REPRESENTATION_KIND_DATA: &str = "data";
const REPRESENTATION_KIND_STRING: &str = "string";
const REPRESENTATION_KIND_PROPERTY_LIST: &str = "plist";

const PROPERTY_LIST_BINARY_FORMAT: usize = 200;

const TYPE_PLAIN_TEXT: [&str; 5] = [
    "public.utf8-plain-text",
    "public.utf16-plain-text",
    "public.text",
    "NSStringPboardType",
    "public.plain-text",
];

const TYPE_FILE_URL: &str = "public.file-url";

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub id: i64,
    pub title: String,
    pub subtitle: String,
}

#[derive(Debug)]
struct ClipboardRepresentation {
    type_identifier: String,
    kind: &'static str,
    payload: Vec<u8>,
}

#[derive(Debug)]
struct ClipboardItem {
    item_index: i64,
    representations: Vec<ClipboardRepresentation>,
}

#[derive(Debug)]
struct ClipboardCapture {
    items: Vec<ClipboardItem>,
    title: String,
    subtitle: String,
}

struct ClipboardHistory {
    connection: Connection,
    last_change_count: isize,
}

thread_local! {
    static HISTORY: RefCell<Option<ClipboardHistory>> = RefCell::new(None);
}

pub fn init_history() -> Result<(), String> {
    let database_path = history_database_path()?;
    let mut history = ClipboardHistory {
        connection: Connection::open(database_path)
            .map_err(|error| format!("打开历史数据库失败: {error}"))?,
        last_change_count: current_change_count()?,
    };

    initialize_schema(&history.connection)?;

    if history_entry_count(&history.connection)? == 0 {
        if let Some(capture) = read_current_pasteboard_capture()? {
            insert_capture(&mut history.connection, &capture)?;
        }
    }

    HISTORY.with(|slot| {
        *slot.borrow_mut() = Some(history);
    });

    Ok(())
}

pub fn poll_clipboard_change() -> Result<bool, String> {
    HISTORY.with(|slot| {
        let mut state = slot.borrow_mut();
        let history = state
            .as_mut()
            .ok_or_else(|| "剪切板历史尚未初始化".to_owned())?;

        let change_count = current_change_count()?;
        if change_count == history.last_change_count {
            return Ok(false);
        }

        history.last_change_count = change_count;

        let Some(capture) = read_current_pasteboard_capture()? else {
            return Ok(false);
        };

        insert_capture(&mut history.connection, &capture)?;
        debug!("clipboard history captured: {}", capture.title);
        Ok(true)
    })
}

pub fn recent_entries(limit: usize) -> Result<Vec<HistoryEntry>, String> {
    HISTORY.with(|slot| {
        let state = slot.borrow();
        let history = state
            .as_ref()
            .ok_or_else(|| "剪切板历史尚未初始化".to_owned())?;

        let mut statement = history
            .connection
            .prepare(
                "SELECT id, title, subtitle
                 FROM clipboard_entries
                 ORDER BY id DESC
                 LIMIT ?1",
            )
            .map_err(|error| format!("查询历史条目失败: {error}"))?;

        let rows = statement
            .query_map(params![limit as i64], |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    subtitle: row.get(2)?,
                })
            })
            .map_err(|error| format!("读取历史条目失败: {error}"))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row.map_err(|error| format!("解析历史条目失败: {error}"))?);
        }

        Ok(entries)
    })
}

pub fn paste_entry(entry_id: i64) -> Result<(), String> {
    HISTORY.with(|slot| {
        let mut state = slot.borrow_mut();
        let history = state
            .as_mut()
            .ok_or_else(|| "剪切板历史尚未初始化".to_owned())?;

        let items = load_entry_payload(&history.connection, entry_id)?;
        if items.is_empty() {
            return Err(format!("未找到可恢复的剪切板条目: {entry_id}"));
        }

        history.last_change_count = write_capture_to_pasteboard(&items)?;
        paste::commit_current_clipboard()
    })
}

fn history_database_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|error| format!("读取 HOME 失败: {error}"))?;
    let directory = Path::new(&home)
        .join("Library")
        .join("Application Support")
        .join("cococa-clip");

    fs::create_dir_all(&directory).map_err(|error| format!("创建历史数据库目录失败: {error}"))?;

    Ok(directory.join("clipboard-history.sqlite3"))
}

fn initialize_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS clipboard_entries (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 created_at INTEGER NOT NULL,
                 title TEXT NOT NULL,
                 subtitle TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS clipboard_items (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 entry_id INTEGER NOT NULL REFERENCES clipboard_entries(id) ON DELETE CASCADE,
                 item_index INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS clipboard_representations (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 item_id INTEGER NOT NULL REFERENCES clipboard_items(id) ON DELETE CASCADE,
                 type_identifier TEXT NOT NULL,
                 representation_kind TEXT NOT NULL,
                 payload BLOB NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_clipboard_items_entry_id
                 ON clipboard_items(entry_id, item_index);
             CREATE INDEX IF NOT EXISTS idx_clipboard_representations_item_id
                 ON clipboard_representations(item_id);",
        )
        .map_err(|error| format!("初始化历史数据库表失败: {error}"))
}

fn history_entry_count(connection: &Connection) -> Result<i64, String> {
    connection
        .query_row("SELECT COUNT(*) FROM clipboard_entries", [], |row| {
            row.get(0)
        })
        .map_err(|error| format!("读取历史条目数量失败: {error}"))
}

fn insert_capture(connection: &mut Connection, capture: &ClipboardCapture) -> Result<i64, String> {
    let transaction = connection
        .transaction()
        .map_err(|error| format!("创建历史事务失败: {error}"))?;

    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("计算时间戳失败: {error}"))?
        .as_secs() as i64;

    transaction
        .execute(
            "INSERT INTO clipboard_entries (created_at, title, subtitle) VALUES (?1, ?2, ?3)",
            params![created_at, capture.title, capture.subtitle],
        )
        .map_err(|error| format!("写入历史条目失败: {error}"))?;

    let entry_id = transaction.last_insert_rowid();

    for item in &capture.items {
        transaction
            .execute(
                "INSERT INTO clipboard_items (entry_id, item_index) VALUES (?1, ?2)",
                params![entry_id, item.item_index],
            )
            .map_err(|error| format!("写入历史 item 失败: {error}"))?;
        let item_id = transaction.last_insert_rowid();

        for representation in &item.representations {
            transaction
                .execute(
                    "INSERT INTO clipboard_representations (
                        item_id,
                        type_identifier,
                        representation_kind,
                        payload
                    ) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        item_id,
                        representation.type_identifier,
                        representation.kind,
                        representation.payload,
                    ],
                )
                .map_err(|error| format!("写入历史表示失败: {error}"))?;
        }
    }

    transaction
        .commit()
        .map_err(|error| format!("提交历史事务失败: {error}"))?;

    Ok(entry_id)
}

fn load_entry_payload(
    connection: &Connection,
    entry_id: i64,
) -> Result<Vec<ClipboardItem>, String> {
    let mut item_statement = connection
        .prepare(
            "SELECT id, item_index
             FROM clipboard_items
             WHERE entry_id = ?1
             ORDER BY item_index ASC",
        )
        .map_err(|error| format!("准备 item 查询失败: {error}"))?;

    let item_rows = item_statement
        .query_map(params![entry_id], |row| {
            let item_id: i64 = row.get(0)?;
            let item_index: i64 = row.get(1)?;
            Ok((item_id, item_index))
        })
        .map_err(|error| format!("查询 item 数据失败: {error}"))?;

    let mut items = Vec::new();
    for item_row in item_rows {
        let (item_id, item_index) =
            item_row.map_err(|error| format!("解析 item 数据失败: {error}"))?;

        let mut representation_statement = connection
            .prepare(
                "SELECT type_identifier, representation_kind, payload
                 FROM clipboard_representations
                 WHERE item_id = ?1
                 ORDER BY id ASC",
            )
            .map_err(|error| format!("准备表示查询失败: {error}"))?;

        let representation_rows = representation_statement
            .query_map(params![item_id], |row| {
                Ok(ClipboardRepresentation {
                    type_identifier: row.get(0)?,
                    kind: match row.get::<_, String>(1)?.as_str() {
                        REPRESENTATION_KIND_STRING => REPRESENTATION_KIND_STRING,
                        REPRESENTATION_KIND_PROPERTY_LIST => REPRESENTATION_KIND_PROPERTY_LIST,
                        _ => REPRESENTATION_KIND_DATA,
                    },
                    payload: row.get(2)?,
                })
            })
            .map_err(|error| format!("查询表示数据失败: {error}"))?;

        let mut representations = Vec::new();
        for representation in representation_rows {
            representations
                .push(representation.map_err(|error| format!("解析表示数据失败: {error}"))?);
        }

        items.push(ClipboardItem {
            item_index,
            representations,
        });
    }

    Ok(items)
}

fn read_current_pasteboard_capture() -> Result<Option<ClipboardCapture>, String> {
    let pasteboard = general_pasteboard()?;
    let items: id = unsafe { msg_send![pasteboard, pasteboardItems] };
    if items == nil {
        return Ok(None);
    }

    let count = nsarray_count(items);
    if count == 0 {
        return Ok(None);
    }

    let mut capture_items = Vec::new();
    let mut preview_text = None;
    let mut file_names = Vec::new();
    let mut has_image = false;
    let mut first_types = Vec::new();
    let mut total_representations = 0_usize;

    for item_index in 0..count {
        let item = nsarray_object(items, item_index);
        if item == nil {
            continue;
        }

        let types: id = unsafe { msg_send![item, types] };
        if types == nil {
            continue;
        }

        let mut representations = Vec::new();

        for type_index in 0..nsarray_count(types) {
            let type_identifier_ns = nsarray_object(types, type_index);
            let Some(type_identifier) = nsstring_to_string(type_identifier_ns) else {
                continue;
            };

            if first_types.len() < 3 {
                first_types.push(type_identifier.clone());
            }

            if preview_text.is_none() && is_text_type(&type_identifier) {
                preview_text = string_for_type(item, type_identifier_ns);
            }

            if is_file_url_type(&type_identifier) {
                if let Some(file_url) = string_for_type(item, type_identifier_ns) {
                    file_names.push(file_name_from_url(&file_url));
                }
            }

            if is_image_type(&type_identifier) {
                has_image = true;
            }

            if let Some(data_representation) = data_for_type(item, type_identifier_ns) {
                total_representations += 1;
                representations.push(ClipboardRepresentation {
                    type_identifier,
                    kind: REPRESENTATION_KIND_DATA,
                    payload: data_representation,
                });
                continue;
            }

            if let Some(string_representation) = string_for_type(item, type_identifier_ns) {
                total_representations += 1;
                representations.push(ClipboardRepresentation {
                    type_identifier,
                    kind: REPRESENTATION_KIND_STRING,
                    payload: string_representation.into_bytes(),
                });
                continue;
            }

            if let Some(property_list_representation) =
                property_list_data_for_type(item, type_identifier_ns)?
            {
                total_representations += 1;
                representations.push(ClipboardRepresentation {
                    type_identifier,
                    kind: REPRESENTATION_KIND_PROPERTY_LIST,
                    payload: property_list_representation,
                });
            }
        }

        if !representations.is_empty() {
            capture_items.push(ClipboardItem {
                item_index: item_index as i64,
                representations,
            });
        }
    }

    if capture_items.is_empty() {
        return Ok(None);
    }

    let title;
    let subtitle;
    if let Some(text) = preview_text {
        title = truncate_preview(&normalize_preview(&text), 42);
        subtitle = format!(
            "文本 · {} 项 · {} 个原始表示",
            capture_items.len(),
            total_representations
        );
    } else if !file_names.is_empty() {
        title = if file_names.len() == 1 {
            file_names[0].clone()
        } else {
            format!("{} 个文件", file_names.len())
        };
        subtitle = truncate_preview(&format!("文件 · {}", file_names.join("、")), 54);
    } else if has_image {
        title = if capture_items.len() == 1 {
            "图片".to_owned()
        } else {
            format!("{} 个图片项目", capture_items.len())
        };
        subtitle = format!("图片 · {} 个原始表示", total_representations);
    } else {
        title = "其他剪切板内容".to_owned();
        subtitle = truncate_preview(&first_types.join(" · "), 54);
    }

    Ok(Some(ClipboardCapture {
        items: capture_items,
        title,
        subtitle,
    }))
}

fn write_capture_to_pasteboard(items: &[ClipboardItem]) -> Result<isize, String> {
    let pasteboard = general_pasteboard()?;

    unsafe {
        let _: isize = msg_send![pasteboard, clearContents];
    }

    let array: id = unsafe { msg_send![class!(NSMutableArray), alloc] };
    let array: id = unsafe { msg_send![array, init] };
    if array == nil {
        return Err("创建 NSMutableArray 失败".to_owned());
    }

    for item in items {
        let pasteboard_item: id = unsafe { msg_send![class!(NSPasteboardItem), alloc] };
        let pasteboard_item: id = unsafe { msg_send![pasteboard_item, init] };
        if pasteboard_item == nil {
            return Err("创建 NSPasteboardItem 失败".to_owned());
        }

        for representation in &item.representations {
            let type_identifier =
                unsafe { NSString::alloc(nil).init_str(&representation.type_identifier) };

            let success: BOOL = match representation.kind {
                REPRESENTATION_KIND_STRING => {
                    let text = String::from_utf8_lossy(&representation.payload).to_string();
                    let ns_text = unsafe { NSString::alloc(nil).init_str(&text) };
                    unsafe {
                        msg_send![pasteboard_item, setString: ns_text forType: type_identifier]
                    }
                }
                REPRESENTATION_KIND_PROPERTY_LIST => {
                    let data = nsdata_from_bytes(&representation.payload)?;
                    let mut format: usize = 0;
                    let mut error: id = nil;
                    let property_list: id = unsafe {
                        msg_send![
                            class!(NSPropertyListSerialization),
                            propertyListWithData: data
                            options: 0usize
                            format: &mut format
                            error: &mut error
                        ]
                    };
                    if property_list == nil {
                        return Err("恢复 property list 数据失败".to_owned());
                    }
                    unsafe {
                        msg_send![pasteboard_item, setPropertyList: property_list forType: type_identifier]
                    }
                }
                _ => {
                    let data = nsdata_from_bytes(&representation.payload)?;
                    unsafe { msg_send![pasteboard_item, setData: data forType: type_identifier] }
                }
            };

            if !success {
                return Err(format!(
                    "写回剪切板类型失败: {}",
                    representation.type_identifier
                ));
            }
        }

        unsafe {
            let _: () = msg_send![array, addObject: pasteboard_item];
        }
    }

    let written: bool = unsafe { msg_send![pasteboard, writeObjects: array] };
    if !written {
        return Err("写回系统剪切板失败".to_owned());
    }

    current_change_count()
}

fn current_change_count() -> Result<isize, String> {
    let pasteboard = general_pasteboard()?;
    let change_count: isize = unsafe { msg_send![pasteboard, changeCount] };
    Ok(change_count)
}

fn general_pasteboard() -> Result<id, String> {
    unsafe {
        let pasteboard: id = msg_send![class!(NSPasteboard), generalPasteboard];
        if pasteboard == nil {
            return Err("无法访问系统剪切板".to_owned());
        }

        Ok(pasteboard)
    }
}

fn nsarray_count(array: id) -> usize {
    unsafe { msg_send![array, count] }
}

fn nsarray_object(array: id, index: usize) -> id {
    unsafe { msg_send![array, objectAtIndex: index] }
}

fn nsstring_to_string(value: id) -> Option<String> {
    if value == nil {
        return None;
    }

    unsafe {
        let utf8: *const std::os::raw::c_char = msg_send![value, UTF8String];
        if utf8.is_null() {
            return None;
        }

        Some(CStr::from_ptr(utf8).to_string_lossy().into_owned())
    }
}

fn data_for_type(item: id, type_identifier: id) -> Option<Vec<u8>> {
    unsafe {
        let data: id = msg_send![item, dataForType: type_identifier];
        nsdata_to_vec(data)
    }
}

fn string_for_type(item: id, type_identifier: id) -> Option<String> {
    unsafe {
        let value: id = msg_send![item, stringForType: type_identifier];
        nsstring_to_string(value)
    }
}

fn property_list_data_for_type(item: id, type_identifier: id) -> Result<Option<Vec<u8>>, String> {
    unsafe {
        let property_list: id = msg_send![item, propertyListForType: type_identifier];
        if property_list == nil {
            return Ok(None);
        }

        let mut error: id = nil;
        let data: id = msg_send![
            class!(NSPropertyListSerialization),
            dataWithPropertyList: property_list
            format: PROPERTY_LIST_BINARY_FORMAT
            options: 0usize
            error: &mut error
        ];

        if data == nil {
            return Err("序列化 property list 失败".to_owned());
        }

        Ok(nsdata_to_vec(data))
    }
}

fn nsdata_to_vec(data: id) -> Option<Vec<u8>> {
    if data == nil {
        return None;
    }

    unsafe {
        let length: usize = msg_send![data, length];
        let bytes: *const u8 = msg_send![data, bytes];
        if bytes.is_null() {
            return None;
        }

        Some(slice::from_raw_parts(bytes, length).to_vec())
    }
}

fn nsdata_from_bytes(bytes: &[u8]) -> Result<id, String> {
    unsafe {
        let data: id = msg_send![
            class!(NSData),
            dataWithBytes: bytes.as_ptr()
            length: bytes.len()
        ];
        if data == nil {
            return Err("创建 NSData 失败".to_owned());
        }

        Ok(data)
    }
}

fn is_text_type(type_identifier: &str) -> bool {
    TYPE_PLAIN_TEXT
        .iter()
        .any(|candidate| type_identifier == *candidate || type_identifier.starts_with(candidate))
}

fn is_file_url_type(type_identifier: &str) -> bool {
    type_identifier == TYPE_FILE_URL || type_identifier.contains("file-url")
}

fn is_image_type(type_identifier: &str) -> bool {
    type_identifier.contains("image")
        || type_identifier.contains("png")
        || type_identifier.contains("jpeg")
        || type_identifier.contains("tiff")
}

fn normalize_preview(text: &str) -> String {
    text.lines().next().unwrap_or_default().trim().to_owned()
}

fn truncate_preview(text: &str, max_chars: usize) -> String {
    let mut result = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= max_chars {
            result.push('…');
            break;
        }
        result.push(ch);
    }
    result
}

fn file_name_from_url(file_url: &str) -> String {
    let trimmed = file_url.trim_end_matches('/');
    trimmed
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or(file_url)
        .to_owned()
}
