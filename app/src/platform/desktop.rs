//! A desktop window for development (Linux, Vulkan through winit). Not a
//! shipped target: the key file is unsealed, files are read and written from
//! paths typed into the app's own fields.

use std::cell::RefCell;
use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

use crate::gfx::Gpu;
use crate::ui::{App, Pick, Picked, Platform};

struct Desktop {
    clipboard: RefCell<Option<String>>,
    saves: std::path::PathBuf,
    inbox: crate::ui::Inbox,
}

impl Platform for Desktop {
    fn set_clipboard(&self, text: &str) {
        *self.clipboard.borrow_mut() = Some(text.to_owned());
    }
    fn clipboard(&self) -> Option<String> {
        self.clipboard.borrow().clone()
    }
    fn pick_file(&self, purpose: Pick) {
        // Development: the file named by $RUSERBOX_PICK.
        if let Ok(path) = std::env::var("RUSERBOX_PICK")
            && let Ok(data) = std::fs::read(&path)
        {
            let name = std::path::Path::new(&path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            crate::core::lock(&self.inbox).push_back(Picked { purpose, name, data });
        }
    }
    fn save_file(&self, name: &str, data: Vec<u8>) {
        let _ = std::fs::create_dir_all(&self.saves);
        let _ = std::fs::write(self.saves.join(name), data);
    }
    fn keep_alive(&self, _on: bool) {}
    fn system_dark(&self) -> bool {
        true
    }
}

struct Runner {
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    egui: Option<egui_winit::State>,
    ctx: egui::Context,
    app: App,
    platform: Desktop,
}

impl ApplicationHandler for Runner {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let w = Arc::new(el.create_window(Window::default_attributes().with_title("RUSerBox").with_inner_size(winit::dpi::LogicalSize::new(420.0, 860.0))).expect("window"));
        let size = w.inner_size();
        // SAFETY: the window is kept in `self.window` for as long as the GPU.
        let target = unsafe { wgpu::SurfaceTargetUnsafe::from_window(&*w) }.expect("window handle");
        let mut gpu = match pollster::block_on(Gpu::new(Some(target))) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("{e}");
                el.exit();
                return;
            }
        };
        gpu.resize(size.width, size.height);
        self.egui = Some(egui_winit::State::new(self.ctx.clone(), egui::ViewportId::ROOT, &*w, Some(w.scale_factor() as f32), None, None));
        self.gpu = Some(gpu);
        self.window = Some(w);
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let (Some(w), Some(state)) = (self.window.clone(), self.egui.as_mut()) else { return };
        let resp = state.on_window_event(&w, &event);
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) => {
                if let Some(g) = &mut self.gpu {
                    g.resize(s.width, s.height);
                }
                w.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let input = state.take_egui_input(&w);
                let out = self.ctx.run_ui(input, |ui| self.app.update(ui, &self.platform));
                let ppp = out.pixels_per_point;
                let repaint = out.viewport_output.get(&egui::ViewportId::ROOT).map(|v| v.repaint_delay);
                state.handle_platform_output(&w, out.platform_output.clone());
                if let Some(g) = &mut self.gpu {
                    g.render(&self.ctx, out, ppp);
                }
                match repaint {
                    Some(d) if d.is_zero() => w.request_redraw(),
                    Some(d) if d < std::time::Duration::from_secs(3600) => el.set_control_flow(ControlFlow::wait_duration(d)),
                    _ => el.set_control_flow(ControlFlow::Wait),
                }
            }
            _ => {
                if resp.repaint {
                    w.request_redraw();
                }
            }
        }
    }

    fn about_to_wait(&mut self, _el: &ActiveEventLoop) {
        if let Some(w) = &self.window
            && self.ctx.has_requested_repaint()
        {
            w.request_redraw();
        }
    }
}

/// Runs the development window over the store in [dir].
pub fn run(dir: std::path::PathBuf) {
    std::fs::create_dir_all(&dir).expect("data directory");
    let key = super::keyfile::load(&dir, &super::keyfile::Plain).expect("store key");
    let store = Arc::new(sbm_store::Store::open(&dir, &key).expect("store"));
    let el = EventLoop::new().expect("event loop");
    let ctx = egui::Context::default();
    let proxy_ctx = ctx.clone();
    let wake = el.create_proxy();
    ctx.set_request_repaint_callback(move |_| {
        let _ = wake.send_event(());
        let _ = &proxy_ctx;
    });
    let app = App::new(store, ctx.clone());
    let platform = Desktop { clipboard: RefCell::new(None), saves: dir.join("downloads"), inbox: app.inbox.clone() };
    let mut runner = Runner { window: None, gpu: None, egui: None, ctx, app, platform };
    el.run_app(&mut runner).expect("event loop");
}

impl ApplicationHandler<()> for Runner {}
