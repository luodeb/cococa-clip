use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicyAccessory};
use cocoa::base::{NO, YES, id, nil};
use objc::msg_send;

pub fn configure_app() -> id {
    unsafe {
        let app = NSApp();
        app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
        app
    }
}

pub fn terminate_app() {
    unsafe {
        let app = NSApp();
        let _: () = msg_send![app, terminate: nil];
    }
}

pub fn keep_panel_above_apps(panel: id) {
    unsafe {
        let _: () = msg_send![panel, setFloatingPanel: YES];
        let _: () = msg_send![panel, setHidesOnDeactivate: NO];
        let _: () = msg_send![panel, setWorksWhenModal: YES];
        let _: () = msg_send![panel, setCollectionBehavior: (1 << 0) | (1 << 8)];
    }
}
