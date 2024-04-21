use geo::{
    EuclideanDistance, EuclideanLength, InteriorPoint, LineInterpolatePoint, LineIntersection,
    Vector2DOps,
};
use std::collections::BTreeMap;

lazy_static::lazy_static! {
    static ref LINES:[((f64, f64), (f64, f64)); 3]  = {
        let u:(f64, f64) = (-2., 0.);
        let v:(f64, f64) = (-1., -1.);
        let w:(f64, f64) = (1., -1.);
        let x:(f64, f64) = (1. + 0.5_f64.sqrt(), 1. - 0.5_f64.sqrt());
        let y:(f64, f64) = (1. - 0.5_f64.sqrt(), 1. + 0.5_f64.sqrt());
        let z:(f64, f64) = (-1., 3.0_f64.sqrt());
        [(v, w), (x, y), (z, u)]
    };
}
pub const W_PIXELS: usize = 64;
pub const H_PIXELS: usize = 32;
pub const TOTAL_ANGLES: usize = 360;

const CIRCLE_R: f64 = 1.;

type PixelColor = u32;
type PixelXY = (u32, u32);

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct ScreenPixel {
    idx: usize,
    addr: u32,
    pixel: u32,
}

struct PixelZInfo {
    angle: u32,
    // 从底部向下增长
    value: f64,
    pixel: u32,
    screen_pixel: ScreenPixel,
}

type PixelXYMap = BTreeMap<PixelXY, Vec<PixelZInfo>>;

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct ScreenLineAddr {
    screen_idx: usize,
    addr: u32,
}
#[derive(Debug, Copy, Clone)]
struct ScreenLinePixels {
    pixels: [Option<PixelColor>; W_PIXELS],
}

impl Default for ScreenLinePixels {
    fn default() -> Self {
        Self {
            pixels: [None; W_PIXELS],
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ScreenLine {
    pub screen_idx: usize,
    pub addr: u32,
    pub pixels: [Option<PixelColor>; W_PIXELS],
}

pub type AngleMap = BTreeMap<u32, Vec<ScreenLine>>;

fn gen_pixel_xy_map(lines: &[geo::Line]) -> PixelXYMap {
    let mut xy_map = PixelXYMap::default();
    for x in 0..W_PIXELS {
        for y in 0..W_PIXELS {
            for angle in 0..TOTAL_ANGLES {
                let (angle, x, y) = (angle as u32, x as u32, y as u32);
                let Some(z) = cacl_z_pixel(&lines, angle, x, y) else {
                    continue;
                };
                let entry = xy_map.entry((x, y)).or_default();
                entry.push(z);
            }
        }
    }
    for z_info in xy_map.values_mut() {
        z_info.sort_by_key(|v| v.pixel);
    }
    xy_map
}

pub type PixelSurface = Vec<(u32, u32, (u32, PixelColor))>;
pub type FloatSurface = Vec<(f64, f64, f64)>;

pub fn gen_pyramid_surface(low:i32, height: i32) -> PixelSurface {
    let mut pixel_surface = PixelSurface::new();
    for x in 0..64_u32 {
        for y in 0..64_u32 {
            let x_i32 = x as i32 - 32;
            let y_i32 = y as i32 - 32;
            let h = 32 - (x_i32.abs() + y_i32.abs());
            if h < low || h > height {
                continue;
            }
            let z = h.abs() as u32;
            let color = match (x_i32 >= 0, y_i32 >= 0) {
                (true, true) => 0b111,
                (false, true) => 0b001,
                (false, false) => 0b010,
                (true, false) => 0b101,
            };
            // if color != 0b111 {
            //     continue;
            // }
            pixel_surface.push((x, y, (z, color)));
        }
    }
    pixel_surface
}

pub fn gen_pyramid_surface2() -> PixelSurface {
    let mut pixel_surface = PixelSurface::new();
    for x in 0..64_u32 {
        for y in 0..64_u32 {
            let x_i32 = x as i32 - 32;
            let y_i32 = y as i32 - 32;
            let h = 32 - (x_i32.abs() + y_i32.abs());
            if h < 0 {
                // continue;
            }
            let z = h.abs() as u32;
            let color = match (x_i32 >= 0, y_i32 >= 0) {
                (true, true) => 0b111,
                (false, true) => 0b001,
                (false, false) => 0b010,
                (true, false) => 0b101,
            };
            if color != 0b111 && color != 0b001 {
                continue;
            }
            pixel_surface.push((x, y, (z, color)));
        }
    }
    pixel_surface
}

pub fn gen_plane_surface(height: u32) -> PixelSurface {
    let mut pixel_surface = PixelSurface::new();
    for x in 0..64_u32 {
        for y in 0..64_u32 {
            let z = height;
            pixel_surface.push((x, y, (z, 0b111)));
        }
    }
    pixel_surface
}

pub fn gen_cross_plane_surface() -> PixelSurface {
    let mut pixel_surface = PixelSurface::new();
    for x in 0..64_u32 {
        for y in 0..64_u32 {
            let idx = (x / 8) % 3;
            let z = match idx {
                0 => (32, 0b1),
                1 => (16, 0b10),
                _ => (0, 0b101),
            };
            pixel_surface.push((x, y, z));
        }
    }
    pixel_surface
}

pub struct Codec {
    xy_map: PixelXYMap,
}

impl Codec {
    pub fn new() -> Self {
        let lines = LINES.map(|(a, b)| geo::Line::new(a, b));
        let xy_map = gen_pixel_xy_map(&lines);
        Self { xy_map }
    }

    pub fn encode(&self, pixel_surface: &PixelSurface, pixel_offset: i32) -> AngleMap {
        let mut angle_map: BTreeMap<u32, BTreeMap<ScreenLineAddr, ScreenLinePixels>> =
            BTreeMap::new();
        for &(x, y, (z, color)) in pixel_surface {
            let z_info_list = self.xy_map.get(&(x, y)).unwrap();
            let z_info_idx = match z_info_list.binary_search_by_key(&z, |v| v.pixel) {
                Ok(idx) => idx,
                Err(idx) => {
                    if idx == 0 {
                        idx
                    } else if idx >= z_info_list.len() {
                        idx - 1
                    } else {
                        let idx_l = idx - 1;
                        let l = z_info_list.get(idx_l).unwrap();
                        let r = z_info_list.get(idx).unwrap();
                        // let deta_l = z - l.pixel;
                        // let deta_r = r.pixel - z;
                        // if deta_l < deta_r
                        if z * 2 < l.pixel + r.pixel {
                            idx_l
                        } else {
                            idx
                        }
                    }
                }
            };
            let z_info = z_info_list.get(z_info_idx).or(z_info_list.last()).unwrap();
            let entry = angle_map.entry(z_info.angle).or_default();
            let addr = ScreenLineAddr {
                screen_idx: z_info.screen_pixel.idx,
                addr: z_info.screen_pixel.addr,
            };
            let line_pixels = entry.entry(addr).or_default();
            let pixel_idx = z_info.screen_pixel.pixel as usize;
            if let Some(c) = line_pixels.pixels.get_mut(pixel_idx) {
                *c = Some(color);
            } else {
                panic!("{x}, {y}, {z}, pixel_idx {pixel_idx}");
            }
        }
        angle_map
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    v.into_iter()
                        .map(|(k, mut v)| {
                            if pixel_offset > 0 {
                                let offset = pixel_offset as usize;
                                v.pixels.rotate_right(offset);
                                v.pixels.iter_mut().take(offset).for_each(|v| {
                                    v.take();
                                });
                            } else if pixel_offset < 0 {
                                let offset = (-pixel_offset) as usize;
                                v.pixels.rotate_left(offset);
                                v.pixels.iter_mut().rev().take(offset).for_each(|v| {
                                    v.take();
                                });
                            }
                            ScreenLine {
                                screen_idx: k.screen_idx,
                                addr: k.addr,
                                pixels: v.pixels,
                            }
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect()
    }

    pub fn decode(&self, angle_map: AngleMap) -> FloatSurface {
        let mut float_surface = FloatSurface::default();
        for (angle, lines) in angle_map {
            for ScreenLine {
                screen_idx,
                addr,
                pixels,
            } in lines
            {
                for (idx, pixel) in pixels.into_iter().enumerate() {
                    let Some(_pixel) = pixel else { continue };
                    let pixel_z = idx as u32;
                    float_surface.push(cacl_xyz(angle, screen_idx, addr, pixel_z));
                }
            }
        }
        float_surface
    }
}

pub fn pixel_surface_to_float(pixel_surface: &PixelSurface) -> FloatSurface {
    pixel_surface
        .into_iter()
        .map(|&(pixel_x, pixel_y, (pixel_z, _color))| {
            let x = pixel_to_v(pixel_x);
            let y = pixel_to_v(pixel_y);
            let z = pixel_to_h(pixel_z);
            (x, y, z)
        })
        .collect()
}

fn pixel_to_v(p: u32) -> f64 {
    let point_size: f64 = 2. * CIRCLE_R / W_PIXELS as f64;
    p as f64 * point_size + 0.5 * point_size - CIRCLE_R
}

fn v_to_pixel(v: f64) -> Option<u32> {
    let point_size: f64 = 2. * CIRCLE_R / W_PIXELS as f64;
    let v = (v + CIRCLE_R) / point_size - 0.5;
    if v < 0. || v > 63. {
        return None;
    }
    Some(v as u32)
}

fn pixel_to_h(p: u32) -> f64 {
    let point_size: f64 = CIRCLE_R / H_PIXELS as f64;
    (p as f64) * point_size + 0.5 * point_size
}

fn h_to_pixel(h: f64) -> u32 {
    let point_size: f64 = CIRCLE_R / H_PIXELS as f64;
    (h / point_size - 0.5) as u32
}

fn angle_to_v(p: u32) -> f64 {
    p as f64 / TOTAL_ANGLES as f64 * 2. * std::f64::consts::PI
}

fn cacl_z_pixel(lines: &[geo::Line], pixel_angle: u32, x: u32, y: u32) -> Option<PixelZInfo> {
    let angle = angle_to_v(pixel_angle);
    let x = pixel_to_v(x);
    let y = pixel_to_v(y);

    const LEN: f64 = 4. * CIRCLE_R;
    let point_a = geo::Coord::from((LEN * angle.cos(), LEN * angle.sin()));
    let point_a1 = -point_a;
    let point_p = geo::Coord::from((x, y));
    let point_b = point_a + point_p;
    let point_b1 = point_a1 + point_p;
    let line_pb = geo::Line::new(point_p, point_b);
    let mut intersection_info = None;
    for (idx, &line) in lines.iter().enumerate() {
        if let Some(LineIntersection::SinglePoint {
            intersection: points,
            ..
        }) = geo::line_intersection::line_intersection(line, line_pb)
        {
            intersection_info = Some((points, idx, line.start));
            break;
        }
    }
    let (point_s, screen_idx, point_start) = intersection_info?;
    let point_c = geo::Coord::from((point_a.y, -point_a.x));
    let point_c1 = geo::Coord::from((-point_a.y, point_a.x));
    let line_bb1 = geo::Line::new(point_b, point_b1);
    let line_cc1 = geo::Line::new(point_c, point_c1);
    let point_q = geo::line_intersection::line_intersection(line_bb1, line_cc1).unwrap();
    let LineIntersection::SinglePoint {
        intersection: point_q,
        ..
    } = point_q
    else {
        panic!("");
    };
    let len_qs = point_q.euclidean_distance(&point_s);
    let h = CIRCLE_R * 2. - len_qs;

    let pq_len = geo::Line::new(point_p, point_q).euclidean_length();
    let pq = point_p - point_q;
    let sq = point_s - point_q;
    let screen_pixel_h = if pq.dot_product(sq).is_sign_negative() {
        pq_len
    } else {
        -pq_len
    };

    let len_addr = point_start.euclidean_distance(&point_s) - CIRCLE_R;
    let addr = v_to_pixel(len_addr)?;
    let pixel = v_to_pixel(screen_pixel_h)?;
    Some(PixelZInfo {
        angle: pixel_angle,
        value: h,
        pixel: h_to_pixel(h),
        screen_pixel: ScreenPixel {
            idx: screen_idx,
            addr,
            pixel,
        },
    })
}

fn cacl_xyz(angle: u32, screen_idx: usize, addr: u32, pixel_z: u32) -> (f64, f64, f64) {
    let angle = angle_to_v(angle);
    const LEN: f64 = 4. * CIRCLE_R;
    let point_a = geo::Coord::from((LEN * angle.cos(), LEN * angle.sin()));
    let point_a1 = -point_a;
    let point_c = geo::Coord::from((point_a.y, -point_a.x));
    let point_c1 = geo::Coord::from((-point_a.y, point_a.x));
    let line_c_c1 = geo::Line::new(point_c, point_c1);

    let line = LINES[screen_idx];
    let line = geo::Line::new(line.0, line.1);
    let len_start_s = pixel_to_v(addr) + CIRCLE_R;
    let fraction = len_start_s / (CIRCLE_R * 2.);
    let point_s: geo::Coord<_> = line.line_interpolate_point(fraction).unwrap().into();
    let point_s1 = point_s - point_a + point_a1;
    let point_s2 = point_s + point_a;
    let line_s1_s2 = geo::Line::new(point_s1, point_s2);
    let Some(point_q) = geo::line_intersection::line_intersection(line_s1_s2, line_c_c1) else {
        panic!("line_s1_s2 {line_s1_s2:?} line_c_c1 {line_c_c1:?}");
    };
    let LineIntersection::SinglePoint {
        intersection: point_q,
        ..
    } = point_q
    else {
        panic!("");
    };
    // TODO
    let len_pq_with_dir = pixel_to_v(pixel_z);
    let len_pq_abs = len_pq_with_dir.abs();
    let line_o_a1 = geo::Line::new(geo::Coord::zero(), point_a1);
    let point_p1_abs: geo::Coord<_> = line_o_a1
        .line_interpolate_point(len_pq_abs / LEN)
        .unwrap()
        .into();
    let point_p = if len_pq_with_dir.is_sign_positive() {
        point_q + point_p1_abs
    } else {
        point_q - point_p1_abs
    };
    let line_qs = geo::Line::new(point_q, point_s);
    let z = CIRCLE_R * 2. - line_qs.euclidean_length();
    (point_p.x, point_p.y, z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;
    fn print_screen_lines(lines: &[ScreenLine]) {
        let mut buf = "[".to_string();
        for line in lines {
            let mut pixels = "(".to_string();
            for p in line.pixels {
                match p {
                    None => {write!(&mut pixels, "_").unwrap();}
                    Some(c) => {write!(&mut pixels, "{c}").unwrap();}
                }
                pixels.push(',');
            }
            buf.push_str(&format!("{}-{}{}",line.screen_idx, line.addr, pixels));
        }
        buf.push(']');
        println!("{buf}");
    }
    #[test]
    fn it_works() {
        let angle_map1 = Codec::new().encode(&gen_pyramid_surface(0, 30), 0);
        let angle_map2 = Codec::new().encode(&gen_pyramid_surface(0, 32), 0);
        let empty = vec![];
        for angle in 0..TOTAL_ANGLES as u32 {
            let a = angle_map1.get(&angle).unwrap_or(&empty);
            let b = angle_map2.get(&angle).unwrap_or(&empty);
            // println!("angle {} {} {}", a.0, a.1.len(), b.1.len());
            if a == b {
                continue;
            }
            // if a.1 == b.1 {
            //     continue;
            // }
            // for screen_line in b.1.iter() {
            //     if screen_line.screen_idx != 0 {
            //         continue;
            //     }
            //     if screen_line.addr > 24 && screen_line.addr < 40 {
            //         println!("{:?}", screen_line.pixels);
            //     }
            // }
            // println!("diff on angle {}", angle);
            print_screen_lines(a.as_slice());
            print_screen_lines(b.as_slice());
        }
    }
}
