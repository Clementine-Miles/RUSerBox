//! Draws real pages of the app through the Vulkan renderer into an offscreen
//! texture and reads the pixels back. Needs a Vulkan driver (CI and this
//! repository's dev setup use Mesa's lavapipe: `VK_ICD_FILENAMES`); without
//! one it reports the controlled `NoVulkan` error and is skipped.
//! `RUSERBOX_SHOTS=<dir>` writes the frames as PNG.

use std::sync::Arc;

use ruserbox::core::StatusView;
use ruserbox::gfx::{Gpu, GfxError};
use ruserbox::ui::{App, Page, Pick, Platform};

struct Null;
impl Platform for Null {
    fn set_clipboard(&self, _: &str) {}
    fn clipboard(&self) -> Option<String> {
        None
    }
    fn pick_file(&self, _: Pick) {}
    fn save_file(&self, _: &str, _: Vec<u8>) {}
    fn keep_alive(&self, _: bool) {}
    fn system_dark(&self) -> bool {
        true
    }
}

const W: u32 = 432;
const H: u32 = 900;
const PPP: f32 = 1.5;

fn frame(gpu: &mut Gpu, ctx: &egui::Context, app: &mut App) -> Vec<u8> {
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(W as f32 / PPP, H as f32 / PPP))),
        viewports: std::iter::once((egui::ViewportId::ROOT, egui::ViewportInfo { native_pixels_per_point: Some(PPP), ..Default::default() })).collect(),
        ..Default::default()
    };
    // Twice: the first frame lays fonts out.
    let mut first = ctx.run_ui(input(), |ui| app.update(ui, &Null));
    gpu.apply_textures(&mut first.textures_delta);
    let out = ctx.run_ui(input(), |ui| app.update(ui, &Null));
    let tex = gpu.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("shot"),
        size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: gpu.format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    gpu.paint(ctx, out, PPP, [W, H], &view);
    let row = (W * 4).div_ceil(256) * 256;
    let buf = gpu.device().create_buffer(&wgpu::BufferDescriptor { label: None, size: u64::from(row * H), usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ, mapped_at_creation: false });
    let mut enc = gpu.device().create_command_encoder(&Default::default());
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture: &tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo { buffer: &buf, layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: Some(H) } },
        wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
    );
    gpu.queue().submit([enc.finish()]);
    buf.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    gpu.device().poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let data = buf.slice(..).get_mapped_range().expect("mapped");
    let mut px = Vec::with_capacity((W * H * 4) as usize);
    for y in 0..H {
        let s = (y * row) as usize;
        px.extend_from_slice(&data[s..s + (W * 4) as usize]);
    }
    px
}

fn save(name: &str, px: &[u8]) {
    let Ok(dir) = std::env::var("RUSERBOX_SHOTS") else { return };
    std::fs::create_dir_all(&dir).unwrap();
    let f = std::fs::File::create(std::path::Path::new(&dir).join(format!("{name}.png"))).unwrap();
    let mut e = png::Encoder::new(std::io::BufWriter::new(f), W, H);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    e.write_header().unwrap().write_image_data(px).unwrap();
}

fn distinct_colors(px: &[u8]) -> usize {
    let mut set = std::collections::HashSet::new();
    for c in px.chunks(4).step_by(7) {
        set.insert([c[0], c[1], c[2]]);
    }
    set.len()
}

#[test]
fn the_app_renders_through_vulkan() {
    let mut gpu = match pollster::block_on(Gpu::new(None)) {
        Ok(g) => g,
        Err(GfxError::NoVulkan) => {
            eprintln!("skipped: no Vulkan driver");
            return;
        }
        Err(e) => panic!("{e}"),
    };
    assert_eq!(gpu.info.backend, wgpu::Backend::Vulkan, "rendered by {:?}", gpu.info.backend);
    eprintln!("adapter: {} ({})", gpu.info.name, gpu.info.driver);

    let store = Arc::new(sbm_store::Store::in_memory().unwrap());
    let mut s = sbm_store::Server::new_ssh("web-01", "203.0.113.10", 22, "deploy");
    s.auto_connect = false;
    s.tags = vec!["prod".into()];
    store.put_server(&s).unwrap();
    let mut db = sbm_store::Server::new_ssh("db-01", "203.0.113.11", 22, "root");
    db.auto_connect = false;
    store.put_server(&db).unwrap();
    let ctx = egui::Context::default();
    let mut app = App::new(store, ctx.clone());
    for (i, v) in [12.0, 30.0, 25.0, 61.0, 44.0, 38.0, 52.0, 47.0].into_iter().enumerate() {
        app.core.preview_status(
            &s.id,
            StatusView {
                cpu_percent: v,
                cores: vec![v, v / 2.0, v * 1.2, 10.0],
                cpu_brand: Some("AMD EPYC 7B13".into()),
                mem_used: 3_200_000_000 + i as u64 * 10_000_000,
                mem_total: 8_000_000_000,
                disk_used: 40_000_000_000,
                disk_total: 100_000_000_000,
                net_rx_total: i as u64 * 4_000_000,
                net_tx_total: i as u64 * 1_000_000,
                uptime: Some("12 days, 3:04".into()),
                sys: Some("Debian GNU/Linux 13".into()),
                temperature: Some(47.5),
                disks: vec![ruserbox::core::DiskView { mount: "/".into(), path: "/dev/vda1".into(), used: 40_000_000_000, size: 100_000_000_000 }],
                ..Default::default()
            },
        );
    }

    let home = frame(&mut gpu, &ctx, &mut app);
    save("servers", &home);
    assert!(distinct_colors(&home) > 20, "the server list drew something");

    app.push(Page::Detail(s.id.clone()));
    let detail = frame(&mut gpu, &ctx, &mut app);
    save("detail", &detail);
    assert_ne!(home, detail);

    let demo = "\x1b[1;32mdeploy@web-01\x1b[0m:\x1b[1;34m~\x1b[0m$ ls --color\r\n\x1b[1;34mbin\x1b[0m  \x1b[1;34metc\x1b[0m  \x1b[0;32mrun.sh\x1b[0m  notes.txt  中文.md\r\n\x1b[38;5;208m256-color\x1b[0m \x1b[7minverse\x1b[0m \x1b[4munderline\x1b[0m e\u{301}  \u{f303} \u{e0b0}\r\n$ ";
    app.terms.preview("web-01", 60, 20, demo.as_bytes());
    app.push(Page::Terminal);
    let term = frame(&mut gpu, &ctx, &mut app);
    save("terminal", &term);
    assert!(distinct_colors(&term) > 10);

    app.stack.truncate(1);
    app.theme_mode = 1;
    app.push(Page::Settings);
    let light = frame(&mut gpu, &ctx, &mut app);
    save("settings_light", &light);
    let bg = &light[(W * 4 * 5 + 20) as usize..][..3];
    assert!(bg.iter().all(|c| *c > 180), "the light theme is light: {bg:?}");
}
