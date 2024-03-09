mod imp;

use gtk4::glib;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk4::ApplicationWindow, gtk4::Window, gtk4::Widget;
}

impl Window {
    pub fn new<P: glib::IsA<gtk4::Application>>(app: &P) -> Self {
        glib::Object::builder().property("application", app).build()
    }
}
