use arboard::Clipboard;
use cocoa::base::{id, nil};
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode, KeyCode};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use log::debug;
use std::os::raw::c_int;
use std::thread;
use std::time::Duration;

const KEYCODE_V: CGKeyCode = 0x09;
const KEY_INJECTION_STEP_DELAY_MS: u64 = 8;
const TARGET_APP_ACTIVATION_SETTLE_MS: u64 = 10;

fn frontmost_application_pid() -> Result<c_int, String> {
    unsafe {
        let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace == nil {
            return Err("无法访问 NSWorkspace".to_owned());
        }

        let frontmost_app: id = msg_send![workspace, frontmostApplication];
        if frontmost_app == nil {
            return Err("未找到前台应用".to_owned());
        }

        let pid: c_int = msg_send![frontmost_app, processIdentifier];
        if pid <= 0 {
            return Err("前台应用 pid 非法".to_owned());
        }

        Ok(pid)
    }
}

fn activate_application(pid: c_int) -> Result<(), String> {
    unsafe {
        let app_cls = class!(NSRunningApplication);
        let running_app: id = msg_send![app_cls, runningApplicationWithProcessIdentifier: pid];
        if running_app == nil {
            return Err(format!("无法获取 pid={pid} 的 NSRunningApplication"));
        }

        // NSApplicationActivateAllWindows | NSApplicationActivateIgnoringOtherApps
        let options: usize = (1 << 0) | (1 << 1);
        let activated: bool = msg_send![running_app, activateWithOptions: options];
        if !activated {
            return Err(format!("激活目标应用失败 (pid={pid})"));
        }

        Ok(())
    }
}

fn write_to_clipboard(content: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|err| format!("创建系统剪贴板失败: {err}"))?;
    clipboard
        .set_text(content.to_owned())
        .map_err(|err| format!("写入系统剪贴板失败: {err}"))
}

fn create_event_source() -> Result<CGEventSource, String> {
    CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .or_else(|_| CGEventSource::new(CGEventSourceStateID::HIDSystemState))
        .map_err(|_| "创建 CGEventSource 失败（通常是系统权限或会话状态问题）".to_owned())
}

fn build_key_event(
    source: &CGEventSource,
    key_code: CGKeyCode,
    is_key_down: bool,
) -> Result<CGEvent, String> {
    CGEvent::new_keyboard_event(source.clone(), key_code, is_key_down)
        .map_err(|_| "创建键盘事件失败".to_owned())
}

fn post_event(event: &CGEvent, flags: CGEventFlags) {
    event.set_flags(flags);
    event.post(CGEventTapLocation::HID);
}

fn simulate_cmd_v() -> Result<(), String> {
    let source = create_event_source()?;

    let command_down = build_key_event(&source, KeyCode::COMMAND, true)?;
    let command_up = build_key_event(&source, KeyCode::COMMAND, false)?;
    let v_down = build_key_event(&source, KEYCODE_V, true)?;
    let v_up = build_key_event(&source, KEYCODE_V, false)?;

    // 使用完整按键序列提升兼容性：Cmd down -> V down -> V up -> Cmd up。
    post_event(&command_down, CGEventFlags::CGEventFlagCommand);
    thread::sleep(Duration::from_millis(KEY_INJECTION_STEP_DELAY_MS));

    post_event(&v_down, CGEventFlags::CGEventFlagCommand);
    post_event(&v_up, CGEventFlags::CGEventFlagCommand);

    thread::sleep(Duration::from_millis(KEY_INJECTION_STEP_DELAY_MS));
    post_event(&command_up, CGEventFlags::CGEventFlagNull);

    Ok(())
}

fn simulate_cmd_v_legacy() -> Result<(), String> {
    // 兼容兜底：仅发带 command 标记的 V 按键。
    let source = create_event_source()?;
    let key_down = CGEvent::new_keyboard_event(source.clone(), KEYCODE_V, true)
        .map_err(|_| "创建键盘事件失败".to_owned())?;
    let key_up = CGEvent::new_keyboard_event(source, KEYCODE_V, false)
        .map_err(|_| "创建键盘事件失败".to_owned())?;

    post_event(&key_down, CGEventFlags::CGEventFlagCommand);
    post_event(&key_up, CGEventFlags::CGEventFlagCommand);
    Ok(())
}

pub fn commit_text(content: &str) -> Result<(), String> {
    if content.trim().is_empty() {
        debug!("skip paste because input is empty");
        return Ok(());
    }

    debug!("using Cmd+V paste flow (content={:?})", content);
    write_to_clipboard(content)?;

    commit_current_clipboard()
}

pub fn commit_current_clipboard() -> Result<(), String> {
    debug!("using Cmd+V paste flow with current clipboard contents");

    let target_pid = frontmost_application_pid()?;
    if let Err(err) = activate_application(target_pid) {
        debug!("activate target app before Cmd+V failed: {err}");
    } else {
        thread::sleep(Duration::from_millis(TARGET_APP_ACTIVATION_SETTLE_MS));
    }

    if let Err(primary_err) = simulate_cmd_v() {
        debug!("primary key injection failed, fallback to legacy sequence: {primary_err}");
        simulate_cmd_v_legacy()?;
    }

    Ok(())
}
