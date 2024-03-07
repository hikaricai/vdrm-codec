use geo::{EuclideanDistance, EuclideanLength, LineInterpolatePoint, LineIntersection, Vector2DOps};
use std::collections::BTreeMap;

const W_PIXELS: usize = 64;
const H_PIXELS: usize = 32;
const TOTAL_ANGLES: usize = 100;

const CIRCLE_R: f64 = 1.;

type PixelColor = u32;
type PixelXY = (u32, u32);

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
#[derive(Debug, Copy, Clone, Default)]
struct ScreenLinePixels {
    pixels: [Option<PixelColor>; W_PIXELS],
}

#[derive(Debug, Copy, Clone)]
struct ScreenLine {
    screen_idx: usize,
    addr: u32,
    pixels: [Option<PixelColor>; W_PIXELS],
}

type AngleMap = BTreeMap<u32, Vec<ScreenLine>>;

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
        z_info.sort_by_key(|v| v.screen_pixel);
    }
    xy_map
}

type PixelSurface = Vec<(u32, u32, u32)>;
type FloatSurface = Vec<(f64, f64, f64)>;

const POINT_U:(f64, f64) = (-2., 0.);
const POINT_V:(f64, f64) = (-1., -1.);
const POINT_W:(f64, f64) = (1., -1.);
const POINT_X:(f64, f64) = (1. - 0.5_f64.sqrt(), 1. + 0.5_f64.sqrt());
const POINT_Y:(f64, f64) = (1. + 0.5_f64.sqrt(), 1. - 0.5_f64.sqrt());
const POINT_Z:(f64, f64) = (-1., 3.0_f64.sqrt());
const LINES:[((f64, f64), (f64, f64)); 3] = [(POINT_V, POINT_W), (POINT_X, POINT_Y), (POINT_Z, POINT_U)];
pub struct Codec {
    xy_map: PixelXYMap,
}

impl Codec {
    pub fn new() -> Self {
        let lines = LINES.map(|(a, b)| geo::Line::new(a, b));
        let xy_map = gen_pixel_xy_map(&lines);
        Self { xy_map }
    }

    pub fn encode(&self, pixel_surface: PixelSurface) -> AngleMap {
        let mut angle_map: BTreeMap<u32, BTreeMap<ScreenLineAddr, ScreenLinePixels>> =
            BTreeMap::new();
        for (x, y, z) in pixel_surface {
            let z_info_list = self.xy_map.get(&(x, y)).unwrap();
            let z_info_idx = z_info_list
                .binary_search_by_key(&z, |v| v.pixel)
                .unwrap_or_else(|v| v);
            let Some(z_info) = z_info_list.get(z_info_idx).or(z_info_list.last()) else {
                continue;
            };
            let entry = angle_map.entry(z_info.angle).or_default();
            let addr = ScreenLineAddr {
                screen_idx: z_info.screen_pixel.idx,
                addr: z_info.screen_pixel.addr,
            };
            let line_pixels = entry.entry(addr).or_default();
            let pixel_idx = z_info.screen_pixel.pixel as usize;
            if let Some(color) = line_pixels.pixels.get_mut(pixel_idx) {
                *color = Some(1);
            }
        }
        angle_map
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    v.into_iter().map(|(k, v)| ScreenLine {
                        screen_idx: k.screen_idx,
                        addr: k.addr,
                        pixels: v.pixels,
                    }),
                )
            })
            .collect()
    }

    pub fn decode(&self, angle_map: AngleMap) -> FloatSurface {
        let mut float_surface = FloatSurface::default();
        for (angle, LINES) in angle_map {
            for ScreenLine {
                screen_idx,
                addr,
                pixels,
            } in LINES
            {
                for (idx, pixel) in pixels.into_iter().enumerate() {
                    let Some(_pixel) = pixel else { continue };
                }
            }
        }
        float_surface
    }
}

pub fn pixel_surface_to_float(pixel_surface: PixelSurface) -> FloatSurface {
    pixel_surface
        .into_iter()
        .map(|(pixel_x, pixel_y, pixel_z)| {
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

fn v_to_pixel(v: f64) -> u32 {
    let point_size: f64 = 2. * CIRCLE_R / W_PIXELS as f64;
    ((v + CIRCLE_R) / point_size - 0.5) as u32
}

fn pixel_to_h(p: u32) -> f64 {
    let point_size: f64 = CIRCLE_R / H_PIXELS as f64;
    (p as f64) * point_size + 0.5 * point_size
}

fn h_to_pixel(h: f64, total_pixels: usize) -> u32 {
    let point_size: f64 = CIRCLE_R / total_pixels as f64;
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
    Some(PixelZInfo {
        angle: pixel_angle,
        value: h,
        pixel: h_to_pixel(h, H_PIXELS),
        screen_pixel: ScreenPixel {
            idx: screen_idx,
            addr: v_to_pixel(len_addr),
            pixel: v_to_pixel(screen_pixel_h),
        },
    })
}

fn cacl_z(angle: u32, screen_idx: usize, addr: u32, pixel_idx: usize) -> f64 {
    let angle = angle_to_v(angle);
    const LEN: f64 = 4. * CIRCLE_R;
    let point_a = geo::Coord::from((LEN * angle.cos(), LEN * angle.sin()));
    let point_a1 = -point_a;
    let point_c = geo::Coord::from((point_a.y, -point_a.x));
    let point_c1 = geo::Coord::from((-point_a.y, point_a.x));
    let line_c_c1 = geo::Line::new(point_c, point_c1);

    let line = LINES[screen_idx];
    let line = geo::Line::new(line.0, line.1);
    let fraction = addr as f64 / W_PIXELS as f64;
    let len_start_s = pixel_to_v(addr) + CIRCLE_R;
    let fraction = len_start_s / (CIRCLE_R * 2.);
    let point_s: geo::Coord<_> = line.line_interpolate_point(fraction).unwrap().into();
    let point_s1 = point_s - point_a + point_a1;
    let line_s_s1 = geo::Line::new(point_s, point_s1);
    let point_q = geo::line_intersection::line_intersection(line_s_s1, line_c_c1).unwrap();
    let LineIntersection::SinglePoint {
        intersection: point_q,
        ..
    } = point_q
        else {
            panic!("");
        };
    // TODO
    let point_p;
    let h;
    ;0.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
