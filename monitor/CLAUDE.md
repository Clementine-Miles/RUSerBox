# ServerBox Monitor

The headless Rust agent provides metrics, history, alerts, notifications and authenticated server management for the Android app.

- `src/monitoring` and `crates/sbm_native` collect metrics. `crates/sbm_parser` is shared with the SSH connection in the app.
- `src/api/server.rs` registers the HTTP API. Keep the access, TLS and protocol checks on live request paths.
- `migrations/` contains SQLx migrations. Append new migrations so existing database checksums remain valid.

Run `make monitor-dev` for the local API and `cargo test -p server_box_monitor` for the agent tests.
