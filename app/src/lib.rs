//! RUSerBox: server management for Android in Rust.
//!
//! - [`core`]: connections and status, over `sbm_ssh` and
//!   `sbm_monitor_client`, persisted in `sbm_store`.
//! - [`ui`]: the egui interface; [`term`]: the terminal emulator.
//! - [`gfx`]: rendering, Vulkan only. [`platform`]: the Android activity
//!   (and a desktop window for development).

pub mod core;
pub mod fonts;
pub mod gfx;
pub mod i18n;
pub mod platform;
pub mod term;
pub mod theme;
pub mod ui;

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    platform::android::main(app);
}
