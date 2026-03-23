use cocoa::base::{YES, id, nil};
use cocoa::foundation::{NSPoint, NSRect, NSSize, NSString};
use objc::{class, msg_send, sel};

pub fn ns_color(red: u8, green: u8, blue: u8, alpha: f64) -> id {
    unsafe {
        msg_send![
            class!(NSColor),
            colorWithCalibratedRed: red as f64 / 255.0
            green: green as f64 / 255.0
            blue: blue as f64 / 255.0
            alpha: alpha
        ]
    }
}

pub fn style_view(
    view: id,
    background: Option<(u8, u8, u8, f64)>,
    border: Option<(u8, u8, u8, f64, f64)>,
    corner_radius: f64,
) {
    unsafe {
        let _: () = msg_send![view, setWantsLayer: YES];
        let layer: id = msg_send![view, layer];
        if layer == nil {
            return;
        }

        let _: () = msg_send![layer, setCornerRadius: corner_radius];
        let _: () = msg_send![layer, setMasksToBounds: YES];

        if let Some((red, green, blue, alpha)) = background {
            let cg_color: id = msg_send![ns_color(red, green, blue, alpha), CGColor];
            let _: () = msg_send![layer, setBackgroundColor: cg_color];
        }

        if let Some((red, green, blue, alpha, width)) = border {
            let cg_color: id = msg_send![ns_color(red, green, blue, alpha), CGColor];
            let _: () = msg_send![layer, setBorderColor: cg_color];
            let _: () = msg_send![layer, setBorderWidth: width];
        }
    }
}

pub fn style_view_with_shadow(
    view: id,
    background: Option<(u8, u8, u8, f64)>,
    border: Option<(u8, u8, u8, f64, f64)>,
    corner_radius: f64,
    shadow: (u8, u8, u8, f64, f64, f64, f64),
) {
    unsafe {
        style_view(view, background, border, corner_radius);
        let layer: id = msg_send![view, layer];
        if layer == nil {
            return;
        }

        let _: () = msg_send![layer, setMasksToBounds: 0u8];
        let shadow_color: id = msg_send![ns_color(shadow.0, shadow.1, shadow.2, shadow.3), CGColor];
        let _: () = msg_send![layer, setShadowColor: shadow_color];
        let _: () = msg_send![layer, setShadowOpacity: shadow.3 as f32];
        let _: () = msg_send![layer, setShadowRadius: shadow.4];
        let _: () = msg_send![layer, setShadowOffset: NSSize::new(shadow.5, shadow.6)];
    }
}

pub fn build_text_label(
    frame: NSRect,
    text: &str,
    font_size: f64,
    bold: bool,
    color: (u8, u8, u8, f64),
    alignment: usize,
) -> id {
    unsafe {
        let label: id = msg_send![class!(NSTextField), alloc];
        let label: id = msg_send![label, initWithFrame: frame];
        let text_value = NSString::alloc(nil).init_str(text);
        let font: id = if bold {
            msg_send![class!(NSFont), boldSystemFontOfSize: font_size]
        } else {
            msg_send![class!(NSFont), systemFontOfSize: font_size]
        };

        let _: () = msg_send![label, setStringValue: text_value];
        let _: () = msg_send![label, setBezeled: 0u8];
        let _: () = msg_send![label, setDrawsBackground: 0u8];
        let _: () = msg_send![label, setEditable: 0u8];
        let _: () = msg_send![label, setSelectable: 0u8];
        let _: () = msg_send![label, setFont: font];
        let _: () = msg_send![label, setAlignment: alignment];
        let _: () = msg_send![label, setTextColor: ns_color(color.0, color.1, color.2, color.3)];

        label
    }
}

pub fn build_button(
    frame: NSRect,
    title: &str,
    target: id,
    action: objc::runtime::Sel,
    background: (u8, u8, u8, f64),
    border: (u8, u8, u8, f64, f64),
    text_color: (u8, u8, u8, f64),
) -> id {
    unsafe {
        let button: id = msg_send![class!(NSButton), alloc];
        let button: id = msg_send![button, initWithFrame: frame];
        let title = NSString::alloc(nil).init_str(title);

        let _: () = msg_send![button, setTitle: title];
        let _: () = msg_send![button, setBordered: 0u8];
        let _: () = msg_send![button, setTarget: target];
        let _: () = msg_send![button, setAction: action];
        let font: id = msg_send![class!(NSFont), systemFontOfSize: 13.0];
        let _: () = msg_send![button, setFont: font];

        style_view(button, Some(background), Some(border), 14.0);
        let _: () = msg_send![button, setWantsLayer: YES];

        let supports_tint: bool = msg_send![button, respondsToSelector: sel!(setContentTintColor:)];
        if supports_tint {
            let _: () = msg_send![button, setContentTintColor: ns_color(text_color.0, text_color.1, text_color.2, text_color.3)];
        }

        button
    }
}

pub fn build_switch(frame: NSRect, target: id, action: objc::runtime::Sel) -> id {
    unsafe {
        let switch_button: id = msg_send![class!(NSSwitch), alloc];
        let switch_button: id = msg_send![switch_button, initWithFrame: frame];
        let _: () = msg_send![switch_button, setTarget: target];
        let _: () = msg_send![switch_button, setAction: action];

        switch_button
    }
}

pub fn build_divider(frame: NSRect, color: (u8, u8, u8, f64)) -> id {
    unsafe {
        let divider: id = msg_send![class!(NSView), alloc];
        let divider: id = msg_send![divider, initWithFrame: frame];
        style_view(divider, Some(color), None, 0.0);
        divider
    }
}

pub fn set_label_text(label: id, text: &str) {
    unsafe {
        if label == nil {
            return;
        }
        let value = NSString::alloc(nil).init_str(text);
        let _: () = msg_send![label, setStringValue: value];
    }
}

pub fn build_window_title_bar(title: &str, subtitle: &str, width: f64) -> (id, id, id) {
    unsafe {
        let container: id = msg_send![class!(NSView), alloc];
        let container: id = msg_send![
            container,
            initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, 62.0))
        ];
        style_view(container, Some((16, 18, 24, 0.98)), Some((52, 58, 74, 1.0, 1.0)), 0.0);

        let title_label = build_text_label(
            NSRect::new(NSPoint::new(16.0, 31.0), NSSize::new(width - 32.0, 20.0)),
            title,
            16.0,
            true,
            (224, 228, 238, 1.0),
            0,
        );
        let subtitle_label = build_text_label(
            NSRect::new(NSPoint::new(16.0, 10.0), NSSize::new(width - 32.0, 16.0)),
            subtitle,
            11.0,
            false,
            (138, 146, 170, 1.0),
            0,
        );

        let divider = build_divider(
            NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, 1.0)),
            (52, 58, 74, 1.0),
        );

        let _: () = msg_send![container, addSubview: title_label];
        let _: () = msg_send![container, addSubview: subtitle_label];
        let _: () = msg_send![container, addSubview: divider];

        (container, title_label, subtitle_label)
    }
}
