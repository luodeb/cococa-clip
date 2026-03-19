#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotKeyCommand {
    ShowWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotKeyModifier {
    Option,
    Command,
    Control,
    Shift,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HotKeyBinding {
    modifier: HotKeyModifier,
    key_index: usize,
}

const AVAILABLE_KEYS: [(&str, &str, u16); 36] = [
    ("A", "KeyA", 0),
    ("B", "KeyB", 11),
    ("C", "KeyC", 8),
    ("D", "KeyD", 2),
    ("E", "KeyE", 14),
    ("F", "KeyF", 3),
    ("G", "KeyG", 5),
    ("H", "KeyH", 4),
    ("I", "KeyI", 34),
    ("J", "KeyJ", 38),
    ("K", "KeyK", 40),
    ("L", "KeyL", 37),
    ("M", "KeyM", 46),
    ("N", "KeyN", 45),
    ("O", "KeyO", 31),
    ("P", "KeyP", 35),
    ("Q", "KeyQ", 12),
    ("R", "KeyR", 15),
    ("S", "KeyS", 1),
    ("T", "KeyT", 17),
    ("U", "KeyU", 32),
    ("V", "KeyV", 9),
    ("W", "KeyW", 13),
    ("X", "KeyX", 7),
    ("Y", "KeyY", 16),
    ("Z", "KeyZ", 6),
    ("0", "Digit0", 29),
    ("1", "Digit1", 18),
    ("2", "Digit2", 19),
    ("3", "Digit3", 20),
    ("4", "Digit4", 21),
    ("5", "Digit5", 23),
    ("6", "Digit6", 22),
    ("7", "Digit7", 26),
    ("8", "Digit8", 28),
    ("9", "Digit9", 25),
];

#[cfg(target_os = "macos")]
const NS_SHIFT_KEY_MASK: u64 = 1 << 17;
#[cfg(target_os = "macos")]
const NS_CONTROL_KEY_MASK: u64 = 1 << 18;
#[cfg(target_os = "macos")]
const NS_ALTERNATE_KEY_MASK: u64 = 1 << 19;
#[cfg(target_os = "macos")]
const NS_COMMAND_KEY_MASK: u64 = 1 << 20;

impl Default for HotKeyBinding {
    fn default() -> Self {
        Self {
            modifier: HotKeyModifier::Option,
            key_index: 2,
        }
    }
}

impl HotKeyBinding {
    pub fn modifier_label(&self) -> &'static str {
        match self.modifier {
            HotKeyModifier::Option => "Option",
            HotKeyModifier::Command => "Command",
            HotKeyModifier::Control => "Control",
            HotKeyModifier::Shift => "Shift",
        }
    }

    pub fn modifier_symbol(&self) -> &'static str {
        match self.modifier {
            HotKeyModifier::Option => "⌥",
            HotKeyModifier::Command => "⌘",
            HotKeyModifier::Control => "⌃",
            HotKeyModifier::Shift => "⇧",
        }
    }

    pub fn key_label(&self) -> &'static str {
        AVAILABLE_KEYS[self.key_index].0
    }

    pub fn display_text(&self) -> String {
        format!("{} + {}", self.modifier_label(), self.key_label())
    }

    pub fn preview_text(&self) -> String {
        format!("{} + {}", self.modifier_symbol(), self.key_label())
    }
}

#[cfg(target_os = "macos")]
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
#[cfg(target_os = "macos")]
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
#[cfg(target_os = "macos")]
use std::cell::RefCell;
#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
thread_local! {
    static HOTKEY: RefCell<Option<MacHotKey>> = const { RefCell::new(None) };
}

#[cfg(target_os = "macos")]
pub fn init_hotkey() -> Result<(), String> {
    HOTKEY.with(|slot| {
        let mut hotkey = slot.borrow_mut();
        if hotkey.is_none() {
            *hotkey = Some(MacHotKey::new()?);
        }
        Ok(())
    })
}

#[cfg(not(target_os = "macos"))]
pub fn init_hotkey() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn current_binding() -> HotKeyBinding {
    HOTKEY.with(|slot| {
        slot.borrow()
            .as_ref()
            .map(|hotkey| hotkey.binding)
            .unwrap_or_else(load_binding_from_disk)
    })
}

#[cfg(not(target_os = "macos"))]
pub fn current_binding() -> HotKeyBinding {
    HotKeyBinding::default()
}

#[cfg(target_os = "macos")]
pub fn set_binding(binding: HotKeyBinding) -> Result<HotKeyBinding, String> {
    HOTKEY.with(|slot| {
        let mut state = slot.borrow_mut();
        let hotkey = state
            .as_mut()
            .ok_or_else(|| "全局快捷键尚未初始化".to_owned())?;

        hotkey.apply_binding(binding)?;
        Ok(binding)
    })
}

#[cfg(not(target_os = "macos"))]
pub fn set_binding(binding: HotKeyBinding) -> Result<HotKeyBinding, String> {
    Ok(binding)
}

#[cfg(target_os = "macos")]
pub fn binding_from_key_event(key_code: u16, modifier_flags: u64) -> Result<HotKeyBinding, String> {
    let modifier = modifier_from_flags(modifier_flags)?;
    let key_index = AVAILABLE_KEYS
        .iter()
        .position(|(_, _, code)| *code == key_code)
        .ok_or_else(|| "主键仅支持字母 A-Z 或数字 0-9".to_owned())?;

    Ok(HotKeyBinding {
        modifier,
        key_index,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn binding_from_key_event(_: u16, _: u64) -> Result<HotKeyBinding, String> {
    Err("当前平台不支持录制全局快捷键".to_owned())
}

#[cfg(target_os = "macos")]
pub fn poll_command() -> Option<HotKeyCommand> {
    HOTKEY.with(|slot| slot.borrow().as_ref().and_then(MacHotKey::poll_command))
}

#[cfg(not(target_os = "macos"))]
pub fn poll_command() -> Option<HotKeyCommand> {
    None
}

#[cfg(target_os = "macos")]
struct MacHotKey {
    manager: GlobalHotKeyManager,
    wake_hotkey: HotKey,
    binding: HotKeyBinding,
}

#[cfg(target_os = "macos")]
impl MacHotKey {
    fn new() -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|error| format!("Failed to create hotkey manager: {error}"))?;
        let binding = load_binding_from_disk();
        let wake_hotkey = hotkey_from_binding(binding)?;

        manager.register(wake_hotkey).map_err(|error| {
            format!(
                "Failed to register hotkey {}: {error}",
                binding.display_text()
            )
        })?;

        save_binding_to_disk(binding)?;

        Ok(Self {
            manager,
            wake_hotkey,
            binding,
        })
    }

    fn apply_binding(&mut self, binding: HotKeyBinding) -> Result<(), String> {
        let next_hotkey = hotkey_from_binding(binding)?;
        if next_hotkey.id() == self.wake_hotkey.id() {
            self.binding = binding;
            save_binding_to_disk(binding)?;
            return Ok(());
        }

        self.manager
            .unregister(self.wake_hotkey)
            .map_err(|error| format!("Failed to unregister previous hotkey: {error}"))?;

        if let Err(error) = self.manager.register(next_hotkey) {
            let _ = self.manager.register(self.wake_hotkey);
            return Err(format!(
                "Failed to register hotkey {}: {error}",
                binding.display_text()
            ));
        }

        self.wake_hotkey = next_hotkey;
        self.binding = binding;
        save_binding_to_disk(binding)?;
        Ok(())
    }

    fn poll_command(&self) -> Option<HotKeyCommand> {
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.id == self.wake_hotkey.id() && event.state == HotKeyState::Pressed {
                return Some(HotKeyCommand::ShowWindow);
            }
        }

        None
    }
}

#[cfg(target_os = "macos")]
fn hotkey_from_binding(binding: HotKeyBinding) -> Result<HotKey, String> {
    let modifier = match binding.modifier {
        HotKeyModifier::Option => Modifiers::ALT,
        HotKeyModifier::Command => Modifiers::META,
        HotKeyModifier::Control => Modifiers::CONTROL,
        HotKeyModifier::Shift => Modifiers::SHIFT,
    };

    let code = match AVAILABLE_KEYS[binding.key_index].1 {
        "KeyA" => Code::KeyA,
        "KeyB" => Code::KeyB,
        "KeyC" => Code::KeyC,
        "KeyD" => Code::KeyD,
        "KeyE" => Code::KeyE,
        "KeyF" => Code::KeyF,
        "KeyG" => Code::KeyG,
        "KeyH" => Code::KeyH,
        "KeyI" => Code::KeyI,
        "KeyJ" => Code::KeyJ,
        "KeyK" => Code::KeyK,
        "KeyL" => Code::KeyL,
        "KeyM" => Code::KeyM,
        "KeyN" => Code::KeyN,
        "KeyO" => Code::KeyO,
        "KeyP" => Code::KeyP,
        "KeyQ" => Code::KeyQ,
        "KeyR" => Code::KeyR,
        "KeyS" => Code::KeyS,
        "KeyT" => Code::KeyT,
        "KeyU" => Code::KeyU,
        "KeyV" => Code::KeyV,
        "KeyW" => Code::KeyW,
        "KeyX" => Code::KeyX,
        "KeyY" => Code::KeyY,
        "KeyZ" => Code::KeyZ,
        "Digit0" => Code::Digit0,
        "Digit1" => Code::Digit1,
        "Digit2" => Code::Digit2,
        "Digit3" => Code::Digit3,
        "Digit4" => Code::Digit4,
        "Digit5" => Code::Digit5,
        "Digit6" => Code::Digit6,
        "Digit7" => Code::Digit7,
        "Digit8" => Code::Digit8,
        "Digit9" => Code::Digit9,
        _ => return Err("不支持的快捷键键位".to_owned()),
    };

    Ok(HotKey::new(Some(modifier), code))
}

#[cfg(target_os = "macos")]
fn modifier_from_flags(modifier_flags: u64) -> Result<HotKeyModifier, String> {
    let mut modifier = None;

    for (mask, candidate) in [
        (NS_ALTERNATE_KEY_MASK, HotKeyModifier::Option),
        (NS_COMMAND_KEY_MASK, HotKeyModifier::Command),
        (NS_CONTROL_KEY_MASK, HotKeyModifier::Control),
        (NS_SHIFT_KEY_MASK, HotKeyModifier::Shift),
    ] {
        if modifier_flags & mask == 0 {
            continue;
        }

        if modifier.is_some() {
            return Err("暂仅支持一个修饰键，请重新按下组合键".to_owned());
        }

        modifier = Some(candidate);
    }

    modifier.ok_or_else(|| "请至少按住一个修饰键（⌘ / ⌥ / ⌃ / ⇧）".to_owned())
}

#[cfg(target_os = "macos")]
fn load_binding_from_disk() -> HotKeyBinding {
    let path = match binding_config_path() {
        Ok(path) => path,
        Err(_) => return HotKeyBinding::default(),
    };

    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return HotKeyBinding::default(),
    };

    parse_binding_config(&contents).unwrap_or_default()
}

#[cfg(target_os = "macos")]
fn save_binding_to_disk(binding: HotKeyBinding) -> Result<(), String> {
    let path = binding_config_path()?;
    let body = format!(
        "modifier={}\nkey={}\n",
        serialize_modifier(binding.modifier),
        binding.key_label()
    );

    fs::write(path, body).map_err(|error| format!("保存快捷键配置失败: {error}"))
}

#[cfg(target_os = "macos")]
fn parse_binding_config(contents: &str) -> Option<HotKeyBinding> {
    let mut modifier = None;
    let mut key_label = None;

    for line in contents.lines() {
        let (name, value) = line.split_once('=')?;
        match name.trim() {
            "modifier" => modifier = parse_modifier(value.trim()),
            "key" => key_label = Some(value.trim().to_owned()),
            _ => {}
        }
    }

    let modifier = modifier?;
    let key_label = key_label?;
    let key_index = AVAILABLE_KEYS
        .iter()
        .position(|(label, _, _)| *label == key_label)?;

    Some(HotKeyBinding {
        modifier,
        key_index,
    })
}

#[cfg(target_os = "macos")]
fn parse_modifier(value: &str) -> Option<HotKeyModifier> {
    match value {
        "option" => Some(HotKeyModifier::Option),
        "command" => Some(HotKeyModifier::Command),
        "control" => Some(HotKeyModifier::Control),
        "shift" => Some(HotKeyModifier::Shift),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
fn serialize_modifier(modifier: HotKeyModifier) -> &'static str {
    match modifier {
        HotKeyModifier::Option => "option",
        HotKeyModifier::Command => "command",
        HotKeyModifier::Control => "control",
        HotKeyModifier::Shift => "shift",
    }
}

#[cfg(target_os = "macos")]
fn binding_config_path() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|error| format!("读取 HOME 失败: {error}"))?;
    let directory = Path::new(&home)
        .join("Library")
        .join("Application Support")
        .join("cococa-clip");

    fs::create_dir_all(&directory).map_err(|error| format!("创建快捷键配置目录失败: {error}"))?;
    Ok(directory.join("hotkey.conf"))
}
