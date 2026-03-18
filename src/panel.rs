use cocoa::appkit::NSBackingStoreType;
use cocoa::base::{BOOL, NO, YES, id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
use log::debug;
use log::error;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::sync::Once;
use std::thread;
use std::time::Duration;

use crate::app;
use crate::paste;

const NS_WINDOW_STYLE_TITLED: usize = 1 << 0;
const NS_WINDOW_STYLE_CLOSABLE: usize = 1 << 1;
const NS_WINDOW_STYLE_RESIZABLE: usize = 1 << 3;
const NS_WINDOW_STYLE_NONACTIVATING_PANEL: usize = 1 << 7;

const INPUT_TAG: isize = 1001;
const PANEL_HIDE_BEFORE_PASTE_MS: u64 = 40;
const PANEL_RESTORE_AFTER_PASTE_MS: u64 = 40;

pub fn register_controller_class() -> *const Class {
    static ONCE: Once = Once::new();
    static mut CLASS: *const Class = std::ptr::null();

    ONCE.call_once(|| unsafe {
        let superclass = class!(NSObject);
        let mut decl = ClassDecl::new("ClipPanelController", superclass)
            .expect("ClipPanelController class declaration failed");
        decl.add_ivar::<id>("panel");

        decl.add_method(
            sel!(applicationDidFinishLaunching:),
            did_finish_launching as extern "C" fn(&Object, Sel, id),
        );
        decl.add_method(
            sel!(applicationShouldTerminateAfterLastWindowClosed:),
            should_terminate_after_last_window_closed as extern "C" fn(&Object, Sel, id) -> BOOL,
        );
        decl.add_method(sel!(windowWillClose:), window_will_close as extern "C" fn(&Object, Sel, id));
        decl.add_method(sel!(submitText:), submit_text as extern "C" fn(&Object, Sel, id));

        CLASS = decl.register();
    });

    unsafe { CLASS }
}

pub fn new_controller_instance() -> id {
    unsafe {
        let cls = register_controller_class();
        let controller: id = msg_send![cls, new];
        controller
    }
}

extern "C" fn did_finish_launching(this: &Object, _: Sel, _: id) {
    unsafe {
        let panel = build_panel(this as *const Object as id);
        let this_mut = this as *const Object as *mut Object;
        (*this_mut).set_ivar("panel", panel);

        app::keep_panel_above_apps(panel);
        let _: () = msg_send![panel, center];
        let _: () = msg_send![panel, makeKeyAndOrderFront: nil];
        let _: () = msg_send![panel, orderFrontRegardless];
    }
}

extern "C" fn should_terminate_after_last_window_closed(_: &Object, _: Sel, _: id) -> BOOL {
    YES
}

extern "C" fn window_will_close(_: &Object, _: Sel, _: id) {
    app::terminate_app();
}

extern "C" fn submit_text(this: &Object, _: Sel, _: id) {
    unsafe {
        let panel: id = *this.get_ivar("panel");
        if panel == nil {
            return;
        }

        let content_view: id = msg_send![panel, contentView];
        let input_field: id = msg_send![content_view, viewWithTag: INPUT_TAG];
        if input_field == nil {
            return;
        }

        let value: id = msg_send![input_field, stringValue];
        let c_str_ptr: *const c_char = msg_send![value, UTF8String];
        if c_str_ptr.is_null() {
            return;
        }

        let value_text = CStr::from_ptr(c_str_ptr).to_string_lossy();

        // 先隐藏 panel，尽量把焦点留给前台目标应用，再执行粘贴。
        let _: () = msg_send![panel, orderOut: nil];
        thread::sleep(Duration::from_millis(PANEL_HIDE_BEFORE_PASTE_MS));

        if let Err(err) = paste::commit_text(&value_text) {
            error!("粘贴流程失败: {err}");
        }

        thread::sleep(Duration::from_millis(PANEL_RESTORE_AFTER_PASTE_MS));
        let _: () = msg_send![panel, orderFrontRegardless];
        debug!("panel restored after paste flow");

        let empty = NSString::alloc(nil).init_str("");
        let _: () = msg_send![input_field, setStringValue: empty];
    }
}

fn build_panel(controller: id) -> id {
    unsafe {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(560.0, 120.0));
        let style_mask = NS_WINDOW_STYLE_TITLED
            | NS_WINDOW_STYLE_CLOSABLE
            | NS_WINDOW_STYLE_RESIZABLE
            | NS_WINDOW_STYLE_NONACTIVATING_PANEL;

        let panel: id = msg_send![class!(NSPanel), alloc];
        let panel: id = msg_send![
            panel,
            initWithContentRect: frame
            styleMask: style_mask
            backing: NSBackingStoreType::NSBackingStoreBuffered
            defer: NO
        ];

        let title = NSString::alloc(nil).init_str("Clip Input Panel");
        let _: () = msg_send![panel, setTitle: title];
        let _: () = msg_send![panel, setReleasedWhenClosed: NO];
        let _: () = msg_send![panel, setBecomesKeyOnlyIfNeeded: YES];
        let _: () = msg_send![panel, setDelegate: controller];

        let content_view: id = msg_send![panel, contentView];

        let input_frame = NSRect::new(NSPoint::new(20.0, 64.0), NSSize::new(520.0, 28.0));
        let input_field: id = msg_send![class!(NSTextField), alloc];
        let input_field: id = msg_send![input_field, initWithFrame: input_frame];
        let placeholder = NSString::alloc(nil).init_str("输入内容后按 Enter，将模拟 Cmd+V 粘贴到当前前台应用");
        let _: () = msg_send![input_field, setPlaceholderString: placeholder];
        let _: () = msg_send![input_field, setTag: INPUT_TAG];
        let _: () = msg_send![input_field, setTarget: controller];
        let _: () = msg_send![input_field, setAction: sel!(submitText:)];

        let button_frame = NSRect::new(NSPoint::new(440.0, 20.0), NSSize::new(100.0, 28.0));
        let button: id = msg_send![class!(NSButton), alloc];
        let button: id = msg_send![button, initWithFrame: button_frame];
        let button_title = NSString::alloc(nil).init_str("Paste");
        let _: () = msg_send![button, setTitle: button_title];
        let _: () = msg_send![button, setBezelStyle: 1usize];
        let _: () = msg_send![button, setTarget: controller];
        let _: () = msg_send![button, setAction: sel!(submitText:)];

        let hint_frame = NSRect::new(NSPoint::new(20.0, 24.0), NSSize::new(390.0, 17.0));
        let hint_label: id = msg_send![class!(NSTextField), alloc];
        let hint_label: id = msg_send![hint_label, initWithFrame: hint_frame];
        let hint_text = NSString::alloc(nil).init_str("提示：首次使用模拟按键可能需要在系统设置中授予辅助功能权限");
        let _: () = msg_send![hint_label, setStringValue: hint_text];
        let _: () = msg_send![hint_label, setBezeled: NO];
        let _: () = msg_send![hint_label, setDrawsBackground: NO];
        let _: () = msg_send![hint_label, setEditable: NO];
        let _: () = msg_send![hint_label, setSelectable: NO];

        let _: () = msg_send![content_view, addSubview: input_field];
        let _: () = msg_send![content_view, addSubview: button];
        let _: () = msg_send![content_view, addSubview: hint_label];

        panel
    }
}
