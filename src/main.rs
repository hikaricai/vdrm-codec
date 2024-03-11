use gtk4::prelude::*;
use std::collections::BTreeMap;
use vdrm_codec::{ScreenLine, TOTAL_ANGLES, W_PIXELS};

mod gaussian_plot;
mod window;

fn run_app() {
    let application = gtk4::Application::new(
        Some("io.github.plotters-rs.plotters-gtk-demo"),
        Default::default(),
    );

    application.connect_activate(|app| {
        let win = window::Window::new(app);
        win.show();
    });

    application.run();
}


#[derive(Clone, Copy, Default)]
#[repr(packed)]
struct AngleInfo {
    pixel_buf_idx: u16,
    addr_buf_idx: u16,
    lines: u16,
}

fn gen_hub75_data() {
    let mut pixel_surface = vdrm_codec::PixelSurface::new();
    for x in 0..64_u32 {
        for y in 0..64_u32 {
            let z = 16;
            pixel_surface.push((x, y, z));
        }
    }
    let codec = vdrm_codec::Codec::new();
    let angle_map = codec.encode(&pixel_surface);
    let mut pixel_buf: Vec<u8> = vec![];
    let mut addr_buf: Vec<u8> = vec![];
    let mut angle_infos = vec![AngleInfo::default(); TOTAL_ANGLES];
    assert_eq!(std::mem::size_of::<AngleInfo>(), 6);
    const ADDR_MAX: u32 = W_PIXELS as u32 / 2;
    const ADDR_BITS: u32 = ADDR_MAX.ilog2();
    const TOTAL_ADDR_BITS: u32 = ADDR_BITS + 2;

    for (angle, lines) in angle_map {
        let mut addr_map = BTreeMap::<u32, [u8; W_PIXELS]>::new();
        for ScreenLine {
            screen_idx,
            addr,
            pixels,
        } in lines
        {
            let screen_addr = (screen_idx as u32) << ADDR_BITS;
            let real_addr = screen_addr | (addr % ADDR_MAX);
            let color_bits: u8 = if addr >= ADDR_MAX { 0b111 << 3 } else { 0b111 };
            let pixels_entry = addr_map.entry(real_addr).or_insert([0; W_PIXELS]);
            for (pixel, color) in pixels_entry.iter_mut().zip(pixels) {
                *pixel = *pixel | color.map(|_c| color_bits).unwrap_or_default();
            }
        }
        let pixel_buf_idx = pixel_buf.len() as u16;
        let addr_buf_idx = addr_buf.len() as u16;
        let lines = addr_map.len() as u16;
        angle_infos[angle as usize] = AngleInfo{
            pixel_buf_idx,
            addr_buf_idx,
            lines,
        };
        for (addr, pixels) in addr_map {
            let delay_addr = (256_u32 << TOTAL_ADDR_BITS) | addr;
            pixel_buf.extend(pixels);
            addr_buf.extend(delay_addr.to_le_bytes());
        }
    }
    let mut angle_buf: Vec<u8> = vec![];
    for angle_info in angle_infos {
        angle_buf.extend(angle_info.pixel_buf_idx.to_le_bytes());
        angle_buf.extend(angle_info.addr_buf_idx.to_le_bytes());
        angle_buf.extend(angle_info.lines.to_le_bytes());
    }
    std::fs::write("hub75_bufs/angle_buf.bin", angle_buf).unwrap();
    std::fs::write("hub75_bufs/pixel_buf.bin", pixel_buf).unwrap();
    std::fs::write("hub75_bufs/addr_buf.bin", addr_buf).unwrap();
}
fn main() {
    gen_hub75_data();
}
