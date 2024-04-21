use gtk4::prelude::*;
use std::collections::BTreeMap;
use vdrm_codec::{AngleMap, ScreenLine, TOTAL_ANGLES, W_PIXELS};

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

fn gen_pyramid_angle_map(pixel_offset: i32, height: u32) -> AngleMap {
    let pixel_surface = vdrm_codec::gen_pyramid_surface(0, height as i32);
    let codec = vdrm_codec::Codec::new();
    let angle_map = codec.encode(&pixel_surface, pixel_offset);
    angle_map
}

fn gen_plane_angle_map(pixel_offset: i32, height: u32) -> AngleMap {
    let pixel_surface = vdrm_codec::gen_plane_surface(height);
    let codec = vdrm_codec::Codec::new();
    let angle_map = codec.encode(&pixel_surface, pixel_offset);
    angle_map
}

fn gen_cross_plane_angle_map(pixel_offset: i32, height: u32) -> AngleMap {
    let pixel_surface = vdrm_codec::gen_cross_plane_surface();
    let codec = vdrm_codec::Codec::new();
    let angle_map = codec.encode(&pixel_surface, pixel_offset);
    angle_map
}

fn mock_angle_map() -> AngleMap {
    let mut angle_map = AngleMap::new();
    for angle in 0..96 {
        let screen_idx = angle / 32;
        let addr_base = angle % 32;
        let addr = addr_base * 2;
        let mut screen_line1 = ScreenLine {
            screen_idx,
            addr: addr as u32,
            pixels: [None; W_PIXELS],
        };
        screen_line1.pixels[addr] = Some(1);

        let addr = addr_base * 2 + 1;
        let mut screen_line2 = ScreenLine {
            screen_idx,
            addr: addr as u32,
            pixels: [None; W_PIXELS],
        };
        screen_line2.pixels[addr] = Some(1);
        angle_map.insert(angle as u32, vec![screen_line1, screen_line2]);
    }
    angle_map
}

fn mock_angle_map2() -> AngleMap {
    let plane0: Vec<_> = (0..W_PIXELS)
        .filter(|addr| (*addr / 8) % 2 == 0)
        .map(|addr| ScreenLine {
            screen_idx: 0,
            addr: addr as u32,
            pixels: [Some(1); W_PIXELS],
        })
        .collect();

    let plane1: Vec<_> = (0..W_PIXELS)
        .filter(|addr| (*addr / 8) % 2 == 0)
        .map(|addr| ScreenLine {
            screen_idx: 1,
            addr: addr as u32,
            pixels: [Some(0b10); W_PIXELS],
        })
        .collect();

    let plane2: Vec<_> = (0..W_PIXELS)
        .filter(|addr| (*addr / 8) % 2 != 0)
        .map(|addr| ScreenLine {
            screen_idx: 2,
            addr: addr as u32,
            pixels: [Some(0b101); W_PIXELS],
        })
        .collect();

    AngleMap::from([
        (TOTAL_ANGLES as u32 * 3 / 4, plane0),
        (TOTAL_ANGLES as u32 / 8, plane1),
        (TOTAL_ANGLES as u32 * 11 / 24, plane2),
    ])
}

fn gen_hub75_data(angle_map: AngleMap) {
    let mut pixel_buf: Vec<u8> = vec![];
    let mut addr_buf: Vec<u8> = vec![];
    let mut angle_infos = vec![AngleInfo::default(); TOTAL_ANGLES];
    assert_eq!(std::mem::size_of::<AngleInfo>(), 6);
    const ADDR_MAX: u32 = W_PIXELS as u32 / 2;
    const ADDR_BITS: u32 = ADDR_MAX.ilog2();
    const REAL_ADDR_BITS: u32 = ADDR_BITS + 3;
    const TOTAL_ADDR_BITS: u32 = ADDR_BITS + REAL_ADDR_BITS;

    for (angle, lines) in angle_map {
        let mut addr_map = BTreeMap::<u32, [u8; W_PIXELS]>::new();
        for ScreenLine {
            screen_idx,
            addr,
            pixels,
        } in lines
        {
            let screen_addr = (!(1 << screen_idx) & 0b111) << ADDR_BITS;
            // hub75 delay on addr
            let half_addr = addr % ADDR_MAX;
            let real_addr = screen_addr | half_addr;

            let pixels_entry = addr_map.entry(real_addr).or_insert([0; W_PIXELS]);
            for (pixel, color) in pixels_entry.iter_mut().zip(pixels.into_iter().rev()) {
                *pixel = *pixel
                    | color
                        .map(|c| {
                            let color = c & 0b111;
                            let color_bits = if addr < ADDR_MAX { color } else { color << 3 };
                            color_bits as u8
                        })
                        .unwrap_or_default();
            }
        }
        let pixel_buf_idx = pixel_buf.len() as u16;
        let addr_buf_idx = addr_buf.len() as u16;
        let lines = addr_map.len() as u16;
        angle_infos[angle as usize] = AngleInfo {
            pixel_buf_idx,
            addr_buf_idx,
            lines,
        };
        for (real_addr, pixels) in addr_map {
            let addr = real_addr & 0b11111;
            let delay_addr = (256_u32 << TOTAL_ADDR_BITS) | real_addr << ADDR_BITS | addr;
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
    // let angle_map = mock_angle_map();
    let pixel_offset: i32 = std::env::args()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let height: u32 = std::env::args()
        .nth(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(32);
    let t = std::env::args().nth(3).unwrap_or_default();
    println!("pixel_offset {pixel_offset} height {height} type {t}");
    let angle_map = match t.as_str() {
        "cross" => gen_cross_plane_angle_map(pixel_offset, height),
        "plane" => gen_plane_angle_map(pixel_offset, height),
        "mock" => mock_angle_map(),
        "mock2" => mock_angle_map2(),
        "pyramid" => gen_pyramid_angle_map(pixel_offset, height),
        _ => gen_plane_angle_map(pixel_offset, height),
    };
    gen_hub75_data(angle_map);
    // run_app();
}
