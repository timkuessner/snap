use gtk::prelude::*;
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, Layer, LayerShell};

fn main() {
    let app = gtk::Application::builder()
        .application_id("com.timkuessner.snap")
        .build();

    app.connect_activate(|app| {
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Snap")
            .build();

        window.init_layer_shell();
        window.set_layer(Layer::Overlay);

        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);

        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);

        let css = gtk::CssProvider::new();
        css.load_from_data("window { background-color: rgba(0, 0, 0, 0.35); }");

        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        window.set_child(Some(&gtk::Label::new(Some(
            "Snap overlay — press Escape to close",
        ))));

        let controller = gtk::EventControllerKey::new();

        let window_for_escape = window.clone();

        controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                window_for_escape.close();
                return gtk::glib::Propagation::Stop;
            }

            gtk::glib::Propagation::Proceed
        });

        window.add_controller(controller);
        window.fullscreen();
        window.present();
    });

    app.run();
}
