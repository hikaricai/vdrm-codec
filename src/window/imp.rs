use gtk4::glib;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

#[derive(Debug, Default, gtk4::CompositeTemplate)]
#[template(file = "ui.xml")]
pub struct Window;

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &'static str = "Window";
    type Type = super::Window;
    type ParentType = gtk4::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        crate::gaussian_plot::GaussianPlot::ensure_type();
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for Window {}
impl WidgetImpl for Window {}
impl WindowImpl for Window {}
impl ApplicationWindowImpl for Window {}
