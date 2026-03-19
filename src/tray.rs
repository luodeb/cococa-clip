#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    ShowWindow,
    Quit,
}

#[cfg(target_os = "macos")]
use std::cell::RefCell;

use crate::icons;

#[cfg(target_os = "macos")]
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem};
#[cfg(target_os = "macos")]
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

#[cfg(target_os = "macos")]
const TRAY_ICON_SVG: &str = include_str!("../assets/icons/list.svg");
#[cfg(target_os = "macos")]
const TRAY_ICON_SIZE: u32 = 18;

#[cfg(target_os = "macos")]
thread_local! {
    static TRAY: RefCell<Option<MacTray>> = const { RefCell::new(None) };
}

#[cfg(target_os = "macos")]
pub fn init_tray() -> Result<(), String> {
    TRAY.with(|slot| {
        let mut tray = slot.borrow_mut();
        if tray.is_none() {
            *tray = Some(MacTray::new()?);
        }
        Ok(())
    })
}

#[cfg(not(target_os = "macos"))]
pub fn init_tray() -> Result<(), String> {
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn poll_command() -> Option<TrayCommand> {
    TRAY.with(|slot| slot.borrow().as_ref().and_then(MacTray::poll_command))
}

#[cfg(not(target_os = "macos"))]
pub fn poll_command() -> Option<TrayCommand> {
    None
}

#[cfg(target_os = "macos")]
pub struct MacTray {
    _tray_icon: TrayIcon,
    show_id: MenuId,
    quit_id: MenuId,
}

#[cfg(target_os = "macos")]
impl MacTray {
    pub fn new() -> Result<Self, String> {
        let menu = Menu::new();

        let show_item = MenuItem::new("显示主窗口", true, None);
        let quit_item = MenuItem::new("退出", true, None);

        menu.append(&show_item)
            .map_err(|error| format!("Failed to append tray show item: {error}"))?;
        menu.append(&quit_item)
            .map_err(|error| format!("Failed to append tray quit item: {error}"))?;

        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("Cococa Clip")
            .with_icon(build_tray_icon()?)
            .with_icon_as_template(true)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .map_err(|error| format!("Failed to create tray icon: {error}"))?;

        Ok(Self {
            _tray_icon: tray_icon,
            show_id: show_item.id().clone(),
            quit_id: quit_item.id().clone(),
        })
    }

    pub fn poll_command(&self) -> Option<TrayCommand> {
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.show_id {
                return Some(TrayCommand::ShowWindow);
            }

            if event.id == self.quit_id {
                return Some(TrayCommand::Quit);
            }
        }

        None
    }
}

#[cfg(target_os = "macos")]
fn build_tray_icon() -> Result<Icon, String> {
    let rgba = icons::render_svg_rgba(TRAY_ICON_SVG, TRAY_ICON_SIZE)?;

    Icon::from_rgba(rgba, TRAY_ICON_SIZE, TRAY_ICON_SIZE)
        .map_err(|error| format!("Failed to build tray icon image: {error}"))
}
