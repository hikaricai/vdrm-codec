use gtk4::prelude::*;

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

fn main() {
    run_app();
}
