#![allow(unexpected_cfgs)]
#![allow(deprecated)]

#[macro_use]
extern crate objc;

mod app;
mod history;
mod hotkey;
mod paste;
mod tray;
mod ui;

use env_logger::Env;
use log::debug;
use objc::msg_send;

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
    debug!("logger initialized with default debug level");

    unsafe {
        let app = app::configure_app();
        let controller = ui::new_controller_instance();
        let _: () = msg_send![app, setDelegate: controller];
        let _: () = msg_send![app, run];
    }
}
