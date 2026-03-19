#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotKeyCommand {
    ShowWindow,
}

#[cfg(target_os = "macos")]
use std::cell::RefCell;

#[cfg(target_os = "macos")]
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
#[cfg(target_os = "macos")]
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

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
pub fn poll_command() -> Option<HotKeyCommand> {
    HOTKEY.with(|slot| slot.borrow().as_ref().and_then(MacHotKey::poll_command))
}

#[cfg(not(target_os = "macos"))]
pub fn poll_command() -> Option<HotKeyCommand> {
    None
}

#[cfg(target_os = "macos")]
struct MacHotKey {
    _manager: GlobalHotKeyManager,
    wake_hotkey: HotKey,
}

#[cfg(target_os = "macos")]
impl MacHotKey {
    fn new() -> Result<Self, String> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|error| format!("Failed to create hotkey manager: {error}"))?;
        let wake_hotkey = HotKey::new(Some(Modifiers::ALT), Code::KeyC);

        manager
            .register(wake_hotkey)
            .map_err(|error| format!("Failed to register Option+C hotkey: {error}"))?;

        Ok(Self {
            _manager: manager,
            wake_hotkey,
        })
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
