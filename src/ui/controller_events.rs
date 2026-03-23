use block::ConcreteBlock;
use cocoa::appkit::NSEventMask;
use cocoa::base::{BOOL, YES, id, nil};
use cocoa::foundation::{NSPoint, NSRect};
use objc::runtime::Object;
use objc::{class, msg_send};

use crate::hotkey;
use crate::ui::controller_state;
use crate::ui::layout;

const HOTKEY_CAPTURE_CANCEL_KEYCODE: u16 = 53;

pub fn install_event_monitors(controller: *mut Object, main_window: id, settings_window: id) {
    unsafe {
        let mask = (NSEventMask::NSLeftMouseDownMask
            | NSEventMask::NSRightMouseDownMask
            | NSEventMask::NSOtherMouseDownMask)
            .bits();

        let global_handler = ConcreteBlock::new(move |_event: id| {
            handle_global_click(main_window, settings_window);
        })
        .copy();

        let global_monitor: id = msg_send![
            class!(NSEvent),
            addGlobalMonitorForEventsMatchingMask: mask
            handler: &*global_handler
        ];

        let local_handler = ConcreteBlock::new(move |event: id| -> id {
            handle_global_click(main_window, settings_window);
            event
        })
        .copy();

        let local_monitor: id = msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: mask
            handler: &*local_handler
        ];

        let key_handler = ConcreteBlock::new(move |event: id| -> id {
            if try_capture_hotkey(main_window, event) {
                return nil;
            }
            event
        })
        .copy();

        let key_monitor: id = msg_send![
            class!(NSEvent),
            addLocalMonitorForEventsMatchingMask: NSEventMask::NSKeyDownMask.bits()
            handler: &*key_handler
        ];

        (*controller).set_ivar("global_click_monitor", global_monitor);
        (*controller).set_ivar("local_click_monitor", local_monitor);
        (*controller).set_ivar("local_key_monitor", key_monitor);
    }
}

fn handle_global_click(main_window: id, settings_window: id) {
    unsafe {
        let point: NSPoint = msg_send![class!(NSEvent), mouseLocation];

        if layout::is_window_visible(settings_window) {
            if point_inside_window(settings_window, point) {
                return;
            }

            let controller = controller_state::controller_from_window(main_window);
            if controller != nil {
                controller_state::hide_settings_window(&*(controller as *const Object));
            }
        }

        if layout::is_window_visible(main_window) && !point_inside_window(main_window, point) {
            let _: () = msg_send![main_window, orderOut: nil];
        }
    }
}

fn point_inside_window(window: id, point: NSPoint) -> bool {
    unsafe {
        if window == nil {
            return false;
        }
        let frame: NSRect = msg_send![window, frame];
        point.x >= frame.origin.x
            && point.x <= frame.origin.x + frame.size.width
            && point.y >= frame.origin.y
            && point.y <= frame.origin.y + frame.size.height
    }
}

fn try_capture_hotkey(main_window: id, event: id) -> bool {
    unsafe {
        if main_window == nil || event == nil {
            return false;
        }

        let controller = controller_state::controller_from_window(main_window);
        if controller == nil {
            return false;
        }

        let controller = &*(controller as *const Object);
        if !controller_state::is_settings_visible(controller)
            || !controller_state::is_hotkey_recording(controller)
        {
            return false;
        }

        let is_repeat: BOOL = msg_send![event, isARepeat];
        if is_repeat == YES {
            return true;
        }

        let key_code: u16 = msg_send![event, keyCode];
        if key_code == HOTKEY_CAPTURE_CANCEL_KEYCODE {
            controller_state::set_hotkey_draft(None);
            controller_state::set_recording_state(controller, false, None);
            controller_state::refresh_hotkey_views(controller);
            return true;
        }

        let modifier_flags: usize = msg_send![event, modifierFlags];
        let binding = match hotkey::binding_from_key_event(key_code, modifier_flags as u64) {
            Ok(binding) => binding,
            Err(message) => {
                controller_state::set_recording_state(controller, true, Some(&message));
                return true;
            }
        };

        if binding == hotkey::current_binding() {
            controller_state::set_hotkey_draft(None);
            controller_state::set_recording_state(
                controller,
                false,
                Some("该组合已是当前绑定，无需保存"),
            );
            controller_state::refresh_hotkey_views(controller);
            return true;
        }

        controller_state::set_hotkey_draft(Some(binding));
        controller_state::set_recording_state(controller, false, None);
        controller_state::refresh_hotkey_views(controller);
        true
    }
}
