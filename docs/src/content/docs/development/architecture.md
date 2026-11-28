---
title: Implementation Architecture
description: How RUSerBox is built — native Rust on Android, Vulkan rendering, and the shared server layer
---

RUSerBox is a Rust workspace. The Android app and the Monitor agent share the
layer that talks to servers; each owns its own state and interface.

```text
┌──────────────────────────────── Android app (app/) ────────────────────────────────┐
│ platform/android.rs   GameActivity loop, touch/keys/IME, surface lifecycle          │
│ gfx.rs                egui → wgpu, Vulkan only (GfxError::NoVulkan otherwise)       │
│ ui/                   pages: servers, status, terminal, files, snippets, keys, …    │
│ term/                 VT emulation (alacritty_terminal) · key encoding · drawing    │
│ core.rs               connections per server, status polling, host key prompts      │
└───────────┬───────────────────────┬──────────────────────────┬─────────────────────┘
            │                       │                          │
   crates/sbm_ssh          crates/sbm_monitor_client     crates/sbm_store
   russh: exec, PTY,       Monitor HTTP + terminal       encrypted store.db
   SFTP, jumps, forwards   WebSocket, pinned TLS         (SQLite3 Multiple Ciphers)
            │                       │
            └──────── crates/sbm_parser ────────┐
              commands, the status script,      │
              parsers (shared with the Monitor) │
                                                ▼
                                   monitor/ (the agent, deployed separately)
```

## Rendering

The interface is egui, tessellated and drawn by wgpu with **only the Vulkan
backend compiled in**. The instance is created for `Backends::VULKAN` and the
selected adapter's backend is checked; a device without a usable Vulkan driver
gets a plain error instead of a fallback renderer. Frames are drawn only when
something asks for one (input, a background task, an animation) and are paced
by FIFO presentation. The surface is created on `InitWindow` and dropped on
`TerminateWindow`; the device outlives it, and a lost device is rebuilt.

## Android integration

`MainActivity` (Java) extends GameActivity and holds no app logic: Rust calls
its methods over JNI for the clipboard, the document picker and saving, window
insets, sealing the store key with the Android Keystore, migrating the key the
Flutter build kept, the foreground service that keeps sessions alive in the
background, and publishing the home-screen widgets' server list. The widgets
themselves (AppWidget/RemoteViews) remain Kotlin.

Text input uses GameActivity's GameTextInput: the IME's composing region is
shown as preedit at the terminal cursor and committed text is sent once.

## Connections

A server is reached over SSH, the Monitor, or both (SSH leads unless the
Monitor is preferred; the other is the fallback). SSH host keys are always
verified: unknown and changed keys are shown to the user and remembered only on
accept. The Monitor client verifies TLS against the web PKI or a reviewed,
pinned leaf certificate, and refuses an agent whose protocol it cannot speak.

## Storage

One encrypted SQLite file, `store.db`, in the directory the Flutter build used,
so an update keeps its servers, keys, snippets and settings. Schema v43 is the
last Flutter release's; v44 removes the AI Agent's data. A newer or older file
than the build understands is refused, never reset.
