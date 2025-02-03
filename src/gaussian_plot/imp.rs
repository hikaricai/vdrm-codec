use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

use std::cell::Cell;
use std::error::Error;
use std::io::Read;
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;

use neocortex::{Cortex, Semaphore};
use plotters::prelude::*;
use plotters_cairo::CairoBackend;

const SCREEN_WIDTH: usize = 256;
const SCREEN_HEIGHT: usize = 192;
const AXES_LEN: f32 = 1.2;

type PointF64 = (f32, f32, f32);

type NdsZBuf = [u64; SCREEN_HEIGHT * SCREEN_WIDTH + 1];

struct Surfaces {
    cortex: Option<Cortex<NdsZBuf, neocortex::Semaphore>>,
    z_frame: Box<NdsZBuf>,
    x_points: Vec<f32>,
    y_points: Vec<f32>,
    updated: bool,
}

impl Surfaces {
    fn new() -> Self {
        let width = SCREEN_WIDTH as f32;
        let height = SCREEN_HEIGHT as f32;
        let x_points: Vec<_> = (0..SCREEN_WIDTH).map(|v| v as f32 / width).collect();
        let y_points: Vec<_> = (0..SCREEN_HEIGHT).map(|v| v as f32 / height).collect();
        let use_shm = true;
        let (cortex, z_frame) = if use_shm {
            let key = 2334;
            let cortex: Cortex<NdsZBuf, Semaphore> = Cortex::attach(key).unwrap();
            let z_frame = Box::new(cortex.read().unwrap());
            (Some(cortex), z_frame)
        } else {
            const LEN: usize = std::mem::size_of::<NdsZBuf>();
            let mut content: [u8; LEN] = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
            let mut file = std::fs::File::open("frames/1738578316").unwrap();
            file.read_exact(&mut content).unwrap();
            let z_frame = unsafe { Box::new(std::mem::transmute(content)) };
            (None, z_frame)
        };

        Self {
            cortex,
            z_frame,
            x_points,
            y_points,
            updated: false,
        }
    }

    fn update(&mut self) -> bool {
        let Some(cortex) = self.cortex.as_ref() else {
            return false;
        };
        let frame = cortex.read().unwrap();
        let seconds = frame[0];
        if self.updated && seconds == self.z_frame[0] {
            return false;
        }
        self.updated = true;
        const LEN: usize = std::mem::size_of::<NdsZBuf>();
        assert_eq!(LEN, SCREEN_HEIGHT * SCREEN_WIDTH * 8 + 8);
        let content: &[u8; LEN] = unsafe { std::mem::transmute(frame.as_ptr()) };
        std::fs::write(format!("frames/{seconds}"), &content).unwrap();
        self.z_frame = Box::new(frame);
        return true;
    }

    fn iter(&self) -> FrameIter<'_> {
        FrameIter {
            p: &self,
            x: 0,
            y: 0,
        }
    }
}

struct FrameIter<'a> {
    p: &'a Surfaces,
    x: usize,
    y: usize,
}

impl<'a> Iterator for FrameIter<'a> {
    type Item = Rectangle<PointF64>;
    fn next(&mut self) -> Option<Self::Item> {
        let step = 1usize;
        if self.y >= SCREEN_HEIGHT {
            return None;
        }
        let idx = self.x + self.y * SCREEN_WIDTH + 1;
        let pixel = self.p.z_frame[idx];
        let abgr = (pixel >> 32) as u32;
        let [r, g, b, _a] = abgr.to_ne_bytes();
        // let z = ((pixel & 0xFFFF) as u32 >> 9) & 0xFFF;
        let mut z = (pixel & 0xFFFF) as u32;
        if abgr & 0x00FF_FFFF == 0 {
            z = 0;
        }
        let coord = (
            self.p.x_points[self.x],
            self.p.y_points[self.y],
            (z as f32) / 0xFFFF as f32,
        );
        const W: f32 = 1. / SCREEN_WIDTH as f32;
        const H: f32 = 1. / SCREEN_HEIGHT as f32;
        let points = [coord, (coord.0 + W, coord.1 + H, coord.2)];
        let mut s: ShapeStyle = RGBColor(r, g, b).into();
        s.filled = true;
        let point = Rectangle::new(points, s);
        self.x += step;
        if self.x >= SCREEN_WIDTH {
            self.x = 0;
            self.y += step;
        }
        return Some(point);
    }
}

static FLOAT_SURFACES: Mutex<Option<Surfaces>> = Mutex::new(None);

#[derive(Debug, Default, glib::Properties)]
#[properties(wrapper_type = super::GaussianPlot)]
pub struct GaussianPlot {
    #[property(get, set, minimum = -1.57, maximum = 1.57)]
    pitch: Cell<f32>,
    #[property(get, set, minimum = 0.0, maximum = std::f32::consts::PI)]
    yaw: Cell<f32>,
    #[property(get, set, minimum = -10.0, maximum = 10.0)]
    mean_x: Cell<f32>,
    #[property(get, set, minimum = -50, maximum = 50)]
    mean_y: Cell<i32>,
    #[property(get, set, minimum = -32, maximum = 32)]
    std_x: Cell<i32>,
    #[property(get, set, minimum = 0, maximum = 63)]
    section_y: Cell<u32>,
}

#[glib::object_subclass]
impl ObjectSubclass for GaussianPlot {
    const NAME: &'static str = "GaussianPlot";
    type Type = super::GaussianPlot;
    type ParentType = gtk4::Widget;
}

impl ObjectImpl for GaussianPlot {
    fn properties() -> &'static [glib::ParamSpec] {
        Self::derived_properties()
    }

    fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        Self::derived_set_property(self, id, value, pspec);
        self.obj().queue_draw();
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        Self::derived_property(self, id, pspec)
    }
}

impl WidgetImpl for GaussianPlot {
    fn snapshot(&self, snapshot: &gtk4::Snapshot) {
        let width = self.obj().width() as u32;
        let height = self.obj().height() as u32;
        if width == 0 || height == 0 {
            return;
        }

        let bounds = gtk4::graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
        let cr = snapshot.append_cairo(&bounds);
        let backend = CairoBackend::new(&cr, (width, height)).unwrap();
        self.plot_pdf(backend).unwrap();
    }
}

impl GaussianPlot {
    fn plot_pdf<'a, DB: DrawingBackend + 'a>(
        &self,
        backend: DB,
    ) -> Result<(), Box<dyn Error + 'a>> {
        static CNT: AtomicUsize = AtomicUsize::new(0);
        let cnt = CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        println!("plot_pdf {cnt}");
        let mut guard = FLOAT_SURFACES.lock().unwrap();
        let surfaces = match guard.as_mut() {
            Some(v) => {
                if !v.update() {
                    // return Ok(());
                }
                v
            }
            None => {
                guard.replace(Surfaces::new());
                guard.as_mut().unwrap()
            }
        };
        let root = backend.into_drawing_area();

        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root).build_cartesian_3d(
            0.0..AXES_LEN,
            0.0..AXES_LEN,
            0.0..AXES_LEN,
        )?;

        chart.with_projection(|mut p| {
            p.pitch = self.pitch.get() as f64;
            p.yaw = self.yaw.get() as f64;
            p.scale = 0.7;
            p.into_matrix() // build the projection matrix
        });

        chart
            .configure_axes()
            .light_grid_style(BLACK.mix(0.15))
            .max_light_lines(3)
            .draw()?;
        let axis_title_style = ("sans-serif", 20, &BLACK).into_text_style(&root);
        chart
            .draw_series(
                [
                    ("x", (AXES_LEN, 0.0, 0.0)),
                    ("y", (0.0, AXES_LEN, 0.0)),
                    ("z", (0.0, 0.0, AXES_LEN)),
                    ("o", (0., 0., 0.)),
                ]
                .map(|(label, position)| Text::new(label, position, &axis_title_style)),
            )
            .unwrap();
        chart.draw_series(surfaces.iter()).unwrap();
        root.present()?;
        Ok(())
    }
}
