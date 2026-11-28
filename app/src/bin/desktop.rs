//! `cargo run -p ruserbox --features desktop`: the app in a desktop window,
//! for development. Data lives in `$RUSERBOX_DATA` (default `./ruserbox-data`).

fn main() {
    env_logger::init();
    let dir = std::env::var_os("RUSERBOX_DATA").map(Into::into).unwrap_or_else(|| std::path::PathBuf::from("ruserbox-data"));
    ruserbox::platform::desktop::run(dir);
}
