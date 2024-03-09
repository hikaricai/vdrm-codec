use gtk4::glib;

mod imp;

glib::wrapper! {
    pub struct GaussianPlot(ObjectSubclass<imp::GaussianPlot>) @extends gtk4::Widget;
}
