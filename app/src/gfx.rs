//! Rendering: egui on wgpu, **Vulkan only**.
//!
//! The instance is created with `Backends::VULKAN` and the crate is built
//! with no other backend, so there is nothing to fall back to: a device
//! without a usable Vulkan adapter gets [`GfxError::NoVulkan`], which the
//! platform layer shows as a plain message. The adapter's backend is checked
//! again after selection and logged, so "it rendered" always means Vulkan.
//!
//! The surface follows the window's life: [`Gpu::attach`] when the platform
//! gives a window (Android `InitWindow`, after a resume or rotation),
//! [`Gpu::detach`] before it is taken away. Device and adapter outlive
//! surfaces; a lost device is rebuilt from the instance on the next attach.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};

#[derive(Debug, thiserror::Error)]
pub enum GfxError {
    #[error("this device has no usable Vulkan driver; RUSerBox needs Vulkan 1.1 to draw")]
    NoVulkan,
    #[error("the Vulkan device could not be created: {0}")]
    Device(String),
    #[error("the window surface could not be created: {0}")]
    Surface(String),
}

pub struct Gpu {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    format: wgpu::TextureFormat,
    surface: Option<(wgpu::Surface<'static>, wgpu::SurfaceConfiguration)>,
    lost: Arc<AtomicBool>,
    pub info: wgpu::AdapterInfo,
}

fn instance() -> wgpu::Instance {
    let mut d = wgpu::InstanceDescriptor::new_without_display_handle();
    d.backends = wgpu::Backends::VULKAN;
    d.flags = wgpu::InstanceFlags::from_build_config();
    wgpu::Instance::new(d)
}

async fn device(adapter: &wgpu::Adapter) -> Result<(wgpu::Device, wgpu::Queue), GfxError> {
    // Downlevel limits: what every Vulkan 1.1 phone GPU offers.
    let limits = wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits());
    adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("ruserbox"),
            required_features: wgpu::Features::empty(),
            required_limits: limits,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|e| GfxError::Device(e.to_string()))
}

impl Gpu {
    /// Picks a Vulkan adapter able to present to [surface] (when given).
    pub async fn new(surface: Option<wgpu::SurfaceTargetUnsafe>) -> Result<Gpu, GfxError> {
        let instance = instance();
        let surface = match surface {
            // SAFETY: the platform layer keeps the window alive until `detach`.
            Some(t) => Some(unsafe { instance.create_surface_unsafe(t) }.map_err(|e| GfxError::Surface(e.to_string()))?),
            None => None,
        };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
                apply_limit_buckets: false,
                compatible_surface: surface.as_ref(),
            })
            .await
            .map_err(|_| GfxError::NoVulkan)?;
        let info = adapter.get_info();
        if info.backend != wgpu::Backend::Vulkan {
            return Err(GfxError::NoVulkan);
        }
        log::info!("Vulkan adapter: {} ({:?}, driver {} {})", info.name, info.device_type, info.driver, info.driver_info);
        let (device, queue) = device(&adapter).await?;
        let lost = Arc::new(AtomicBool::new(false));
        let flag = lost.clone();
        device.set_device_lost_callback(move |reason, msg| {
            log::warn!("Vulkan device lost ({reason:?}): {msg}");
            flag.store(true, Ordering::SeqCst);
        });
        let format = match &surface {
            Some(s) => preferred_format(&s.get_capabilities(&adapter).formats),
            None => wgpu::TextureFormat::Rgba8Unorm,
        };
        let renderer = Renderer::new(&device, format, RendererOptions { msaa_samples: 1, depth_stencil_format: None, dithering: true, predictable_texture_filtering: false });
        let mut gpu = Gpu { instance, adapter, device, queue, renderer, format, surface: None, lost, info };
        if let Some(s) = surface {
            gpu.configure_new(s, 1, 1);
        }
        Ok(gpu)
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    fn configure_new(&mut self, surface: wgpu::Surface<'static>, w: u32, h: u32) {
        let caps = surface.get_capabilities(&self.adapter);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: w.max(1),
            height: h.max(1),
            // FIFO is always supported and paces to the display: frames are
            // drawn only when something changed, and never faster than vsync.
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes.first().copied().unwrap_or(wgpu::CompositeAlphaMode::Auto),
            view_formats: vec![],
        };
        surface.configure(&self.device, &config);
        self.surface = Some((surface, config));
    }

    /// Creates the surface for a new window.
    ///
    /// # Safety
    /// The window behind [target] must stay valid until [`Gpu::detach`].
    pub unsafe fn attach(&mut self, target: wgpu::SurfaceTargetUnsafe, w: u32, h: u32) -> Result<(), GfxError> {
        if self.lost.load(Ordering::SeqCst) {
            self.rebuild_device()?;
        }
        let surface = unsafe { self.instance.create_surface_unsafe(target) }.map_err(|e| GfxError::Surface(e.to_string()))?;
        if !self.adapter.is_surface_supported(&surface) {
            return Err(GfxError::NoVulkan);
        }
        self.configure_new(surface, w, h);
        Ok(())
    }

    /// Drops the surface before its window goes away.
    pub fn detach(&mut self) {
        self.surface = None;
    }

    pub fn has_surface(&self) -> bool {
        self.surface.is_some()
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        if let Some((s, c)) = &mut self.surface
            && (c.width != w.max(1) || c.height != h.max(1))
        {
            c.width = w.max(1);
            c.height = h.max(1);
            s.configure(&self.device, c);
        }
    }

    fn rebuild_device(&mut self) -> Result<(), GfxError> {
        let (device, queue) = futures_block(device(&self.adapter))?;
        let flag = self.lost.clone();
        device.set_device_lost_callback(move |reason, msg| {
            log::warn!("Vulkan device lost ({reason:?}): {msg}");
            flag.store(true, Ordering::SeqCst);
        });
        self.renderer = Renderer::new(&device, self.format, RendererOptions { msaa_samples: 1, depth_stencil_format: None, dithering: true, predictable_texture_filtering: false });
        self.device = device;
        self.queue = queue;
        self.lost.store(false, Ordering::SeqCst);
        Ok(())
    }

    /// Releases cached GPU memory the renderer can rebuild (low memory).
    pub fn trim(&mut self) {
        let _ = self.device.poll(wgpu::PollType::Poll);
    }

    /// Draws one egui frame to the surface. Answers whether it was presented.
    pub fn render(&mut self, ctx: &egui::Context, mut out: egui::FullOutput, pixels_per_point: f32) -> bool {
        if self.lost.load(Ordering::SeqCst) {
            let s = self.surface.take();
            if self.rebuild_device().is_err() {
                out.textures_delta.clear();
                return false;
            }
            if let Some((surface, c)) = s {
                self.configure_new(surface, c.width, c.height);
            }
        }
        if self.surface.is_none() {
            self.apply_textures(&mut out.textures_delta);
            return false;
        }
        let Some((surface, config)) = &self.surface else { return false };
        let frame = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                surface.configure(&self.device, config);
                self.apply_textures(&mut out.textures_delta);
                return false;
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Validation => {
                self.apply_textures(&mut out.textures_delta);
                return false;
            }
        };
        let size = [config.width, config.height];
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.paint(ctx, out, pixels_per_point, size, &view);
        self.queue.present(frame);
        true
    }

    /// Paints into [view] of [size] pixels (the surface's, or an offscreen
    /// texture in tests).
    /// Uploads and frees egui's textures for a frame that is not drawn: the
    /// font atlas changes must not be lost with it.
    pub fn apply_textures(&mut self, delta: &mut egui::TexturesDelta) {
        for (id, deltas) in &delta.set {
            for d in deltas {
                self.renderer.update_texture(&self.device, &self.queue, *id, d);
            }
        }
        for id in &delta.free {
            self.renderer.free_texture(id);
        }
        delta.clear();
    }

    pub fn paint(&mut self, ctx: &egui::Context, mut out: egui::FullOutput, pixels_per_point: f32, size: [u32; 2], view: &wgpu::TextureView) {
        let jobs = ctx.tessellate(out.shapes, pixels_per_point);
        for (id, deltas) in &out.textures_delta.set {
            for delta in deltas {
                self.renderer.update_texture(&self.device, &self.queue, *id, delta);
            }
        }
        let screen = ScreenDescriptor { size_in_pixels: size, pixels_per_point };
        let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("egui") });
        let extra = self.renderer.update_buffers(&self.device, &self.queue, &mut enc, &jobs, &screen);
        {
            let bg = ctx.global_style().visuals.panel_fill;
            let mut pass = enc
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: lin(bg.r()), g: lin(bg.g()), b: lin(bg.b()), a: 1.0 }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.renderer.render(&mut pass, &jobs, &screen);
        }
        self.queue.submit(extra.into_iter().chain(std::iter::once(enc.finish())));
        for id in &out.textures_delta.free {
            self.renderer.free_texture(id);
        }
        out.textures_delta.clear();
    }

    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }
}

fn lin(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// An sRGB format when the surface offers one (egui writes linear colors).
fn preferred_format(formats: &[wgpu::TextureFormat]) -> wgpu::TextureFormat {
    formats.iter().copied().find(|f| f.is_srgb()).or_else(|| formats.first().copied()).unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb)
}

fn futures_block<F: std::future::Future>(f: F) -> F::Output {
    // A tiny executor for the device request, which completes synchronously
    // on native backends.
    use std::task::{Context, Poll, Waker};
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// Blocks on a future during startup (the device request).
pub fn block_on<F: std::future::Future>(f: F) -> F::Output {
    futures_block(f)
}
