use geo::{EuclideanDistance, EuclideanLength, LineIntersection, Vector2DOps};
use std::collections::BTreeMap;
use std::ops::Sub;

const W_PIXELS: usize = 64;
const H_PIXELS: usize = 64;
const TOTAL_ANGLES: usize = 100;

const CIRCLE_R: f64 = 1.;

type PixelColor = u32;
type PixelXY = (u32, u32);

struct PixelZInfo {
    angle: u32,
    h: f64,
    screen_idx: usize,
    addr: u32,
    pixel: u32,
}

impl PixelZInfo {
    fn to_screen_line(&self) -> ScreenLine {
        ScreenLine {
            screen_idx: 0,
            addr: 0,
            pixels: [None; H_PIXELS],
        }
    }
}

type PixelXYMap = BTreeMap<PixelXY, Vec<PixelZInfo>>;

struct ScreenLine {
    screen_idx: usize,
    addr: u32,
    pixels: [Option<PixelColor>; H_PIXELS],
}

type AngleMap = BTreeMap<u32, Vec<ScreenLine>>;

fn gen_pixel_xy_map(lines: &[geo::Line]) -> PixelXYMap {
    let mut xy_map = PixelXYMap::default();
    for x in 0..W_PIXELS {
        for y in 0..W_PIXELS {
            for angle in 0..TOTAL_ANGLES {
                let (angle, x, y) = (angle as u32, x as u32, y as u32);
                let Some(z) = cacl_height(&lines, angle, x, y) else {
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

type PixelSurface = Vec<(u32, u32, u32)>;
type FloatSurface = Vec<(f64, f64, f64)>;

pub fn encode(pixel_surface: PixelSurface) -> AngleMap {
    let point_u = (-2., 0.);
    let point_v = (-1., -1.);
    let point_w = (1., -1.);
    let point_x = (1. - 0.5_f64.sqrt(), 1. + 0.5_f64.sqrt());
    let point_y = (1. + 0.5_f64.sqrt(), 1. - 0.5_f64.sqrt());
    let point_z = (-1., 3.0_f64.sqrt());
    let lines = [(point_v, point_w), (point_x, point_y), (point_z, point_u)];
    let lines = lines.map(|(a, b)| geo::Line::new(a, b));
    let xy_map = gen_pixel_xy_map(&lines);
    let mut angle_map = AngleMap::default();
    for (x, y, z) in pixel_surface {
        let z_info_list = xy_map.get(&(x, y)).unwrap();
        let z_info_idx = z_info_list.binary_search_by_key(&z, |v| v.pixel).unwrap_or_else(|v| v);
        let Some(z_info) = z_info_list.get(z_info_idx).or(z_info_list.last()) else {
            continue;
        };
        let entry = angle_map.entry(z_info.angle).or_default();
        entry.push(z_info.to_screen_line());
    }
    angle_map
}

pub fn pixel_surface_to_float(pixel_surface: PixelSurface) -> FloatSurface {
    vec![]
}

pub fn decode(angle_map: AngleMap) -> FloatSurface {
    vec![]
}

fn pixel_to_v(p: u32, total_pixels: usize) -> f64 {
    let point_size: f64 = 2. * CIRCLE_R / total_pixels as f64;
    p as f64 * point_size + 0.5 * point_size - CIRCLE_R
}

fn v_to_pixel(v: f64, total_pixels: usize) -> u32 {
    let point_size: f64 = 2. * CIRCLE_R / total_pixels as f64;
    ((v + CIRCLE_R) / point_size - 0.5) as u32
}

fn angle_to_v(p: u32, total_angles: usize) -> f64 {
    p as f64 / total_angles as f64 * 2. * std::f64::consts::PI
}

fn cacl_height(lines: &[geo::Line], pixel_angle: u32, x: u32, y: u32) -> Option<PixelZInfo> {
    let angle = angle_to_v(pixel_angle, TOTAL_ANGLES);
    let x = pixel_to_v(x, W_PIXELS);
    let y = pixel_to_v(y, W_PIXELS);

    const LEN: f64 = 4.;
    let point_a = geo::Coord::from((LEN * angle.cos(), LEN * angle.sin()));
    let point_a1 = -point_a;
    let point_p = geo::Coord::from((x, y));
    let point_b = point_a + point_p;
    let point_b1 = point_a1 + point_p;
    let line_pb = geo::Line::new(point_p, point_b);
    let mut intersection_info = None;
    for (idx, &line) in lines.iter().enumerate() {
        if let Some(LineIntersection::SinglePoint { intersection: points, .. }) =
            geo::line_intersection::line_intersection(line, line_pb)
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
    let h = 2. * CIRCLE_R - (CIRCLE_R + len_qs);

    let pq_len = geo::Line::new(point_p, point_q).euclidean_length();
    let pq = point_p - point_q;
    let sq = point_s - point_q;
    let pixel_h = if pq.dot_product(sq).is_sign_negative() {
        CIRCLE_R + pq_len
    } else {
        CIRCLE_R - pq_len
    };

    let len_addr = point_start.euclidean_distance(&point_s);
    Some(PixelZInfo {
        angle: pixel_angle,
        h,
        screen_idx,
        addr: v_to_pixel(len_addr, W_PIXELS),
        pixel: v_to_pixel(pixel_h, H_PIXELS),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}
