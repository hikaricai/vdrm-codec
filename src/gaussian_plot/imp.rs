use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

use std::cell::Cell;
use std::convert::TryFrom;
use std::error::Error;
use std::f64;
use std::sync::Mutex;

use plotters::prelude::*;
use plotters_cairo::CairoBackend;
const AXES_LEN: f64 = 2.;

// lazy_static::lazy_static!{
//     static ref CODEC: vdrm_codec::Codec = vdrm_codec::Codec::new();
//     static ref FLOAT_SURFACES: (vdrm_codec::FloatSurface, vdrm_codec::FloatSurface) = {
//         gen_float_surface()
//     };
// }

struct Surfaces {
    section_y: u32,
    real: vdrm_codec::FloatSurface,
    emu: vdrm_codec::FloatSurface,
}
static FLOAT_SURFACES: Mutex<Option<Surfaces>> = Mutex::new(None);


fn gen_float_surface(section_y: u32) -> Surfaces {
    let mut pixel_surface = vdrm_codec::PixelSurface::new();
    for x in 0..64_u32 {
        for y in 0..64_u32 {
            // let z = ((x as f64 + y as f64).cos() + 1.) / 2.;
            // let z = z * 32.;
            // let z = z as u32;
            // let z = std::cmp::min(z, 31);
            let z = section_y;
            pixel_surface.push((x, y, z));
        }
    }
    let real_float_surface = vdrm_codec::pixel_surface_to_float(&pixel_surface).into_iter().map(|(x, y, z)|(x, z, y)).collect();
    let codec = vdrm_codec::Codec::new();
    let angle_map = codec.encode(&pixel_surface);
    let float_surface = codec.decode(angle_map).into_iter().map(|(x, y, z)|(x, z, y)).collect();
    Surfaces{
        section_y,
        real: real_float_surface,
        emu: float_surface,
    }
}

#[derive(Debug, Default, glib::Properties)]
#[properties(wrapper_type = super::GaussianPlot)]
pub struct GaussianPlot {
    #[property(get, set, minimum = -1.57, maximum = 1.57)]
    pitch: Cell<f64>,
    #[property(get, set, minimum = 0.0, maximum = f64::consts::PI)]
    yaw: Cell<f64>,
    #[property(get, set, minimum = -10.0, maximum = 10.0)]
    mean_x: Cell<f64>,
    #[property(get, set, minimum = -10.0, maximum = 10.0)]
    mean_y: Cell<f64>,
    #[property(get, set, minimum = 0.0, maximum = 10.0)]
    std_x: Cell<f64>,
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
    fn gaussian_pdf(&self, x: f64, y: f64) -> f64 {
        let x_diff = (x - self.mean_x.get()) / self.std_x.get();
        let y_diff = x_diff;
        let exponent = -(x_diff * x_diff + y_diff * y_diff) / 2.0;
        let denom = (2.0 * std::f64::consts::PI / self.std_x.get() / self.std_x.get()).sqrt();
        let gaussian_pdf = 1.0 / denom;
        gaussian_pdf * exponent.exp()
    }

    fn plot_pdf<'a, DB: DrawingBackend + 'a>(
        &self,
        backend: DB,
    ) -> Result<(), Box<dyn Error + 'a>> {
        let root = backend.into_drawing_area();

        root.fill(&WHITE)?;

        let mut chart =
            ChartBuilder::on(&root).build_cartesian_3d(-AXES_LEN..AXES_LEN, -AXES_LEN..AXES_LEN, -AXES_LEN..AXES_LEN)?;

        chart.with_projection(|mut p| {
            p.pitch = self.pitch.get();
            p.yaw = self.yaw.get();
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
                    ("x", (AXES_LEN, -AXES_LEN, -AXES_LEN)),
                    ("y", (-AXES_LEN, AXES_LEN, -AXES_LEN)),
                    ("z", (-AXES_LEN, -AXES_LEN, AXES_LEN)),
                    ("", (0., 0., 0.)),
                ]
                .map(|(label, position)| Text::new(label, position, &axis_title_style)),
            )
            .unwrap();

        let section_y = self.section_y.get();
        let mut guard = FLOAT_SURFACES.lock().unwrap();
        if guard.as_mut().map(|v|v.section_y) != Some(section_y) {
            let surfaces = gen_float_surface(section_y);
            guard.replace(surfaces);
        }
        let surfaces = guard.as_ref().unwrap();
        let total_points: PointSeries<_, _, Circle<_, _>, _> =
            PointSeries::new(surfaces.real.clone(), 1_f64, &BLUE.mix(0.2));
        chart.draw_series(total_points).unwrap();
        let total_points: PointSeries<_, _, Circle<_, _>, _> =
            PointSeries::new(surfaces.emu.clone(), 1_f64, &RED.mix(0.2));
        chart.draw_series(total_points).unwrap();
        // chart.draw_series(
        //     SurfaceSeries::xoz(
        //         (-50..=50).map(|x| x as f64 / 5.0),
        //         (-50..=50).map(|x| x as f64 / 5.0),
        //         |x, y| self.gaussian_pdf(x, y),
        //     )
        //     .style_func(&|&v| (&HSLColor(240.0 / 360.0 - 240.0 / 360.0 * v, 1.0, 0.7)).into()),
        // )?;

        root.present()?;
        Ok(())
    }
}
