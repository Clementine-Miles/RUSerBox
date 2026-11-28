#[cfg(any(target_os = "android", test))]
mod platform {
    pub mod window;
}

#[cfg(target_os = "android")]
mod graphics {
    pub mod surface;
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: android_activity::AndroidApp) {
    use android_activity::{InputStatus, MainEvent, PollEvent};

    let mut window_state = platform::window::WindowState::default();
    let mut running = true;
    while running {
        app.poll_events(None, |event| match event {
            PollEvent::Main(MainEvent::InitWindow { .. }) => window_state.set_window(true),
            PollEvent::Main(MainEvent::TerminateWindow { .. }) => window_state.set_window(false),
            PollEvent::Main(MainEvent::Resume { .. }) => window_state.set_resumed(true),
            PollEvent::Main(MainEvent::Pause | MainEvent::Stop) => window_state.set_resumed(false),
            PollEvent::Main(MainEvent::InputAvailable) => {
                if let Ok(mut events) = app.input_events_iter() {
                    while events.next(|_| InputStatus::Unhandled) {}
                }
            }
            PollEvent::Main(MainEvent::Destroy) => running = false,
            _ => {}
        });
        // Return synchronously from Android's lifecycle callbacks before asking the driver.
        if running
            && window_state.take_probe()
            && let Some(window) = app.native_window()
        {
            // The acquired window stays alive until the probe releases its surface.
            let status = match unsafe { graphics::surface::probe(window.ptr().as_ptr().cast()) } {
                Ok(adapter) => format!("Vulkan surface ready on {adapter}"),
                Err(reason) => format!("Vulkan surface unavailable: {reason}"),
            };
            if let Err(err) = report_status(&app, &status) {
                eprintln!("Could not report surface status: {err}");
            }
        }
    }
}

#[cfg(target_os = "android")]
fn report_status(app: &android_activity::AndroidApp, status: &str) -> jni::errors::Result<()> {
    use jni::{JavaVM, jni_sig, jni_str, objects::JObject, refs::Global};

    // attach_current_thread scopes the local frame and translates pending Java exceptions.
    JavaVM::singleton()?.attach_current_thread(|env| {
        let raw = app.activity_as_ptr() as jni::sys::jobject;
        // AndroidApp owns this global reference for the lifetime of this callback.
        let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw)? };
        let text = env.new_string(status)?;
        env.call_method(
            activity,
            jni_str!("reportSurfaceStatus"),
            jni_sig!((java.lang.String) -> void),
            &[(&text).into()],
        )?;
        Ok(())
    })
}
