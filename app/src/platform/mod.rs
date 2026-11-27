//! Platform glue: the Android activity, and a desktop window for development.

#[cfg(target_os = "android")]
pub mod android;
#[cfg(all(feature = "desktop", not(target_os = "android")))]
pub mod desktop;
pub mod keyfile;
