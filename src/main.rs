use gtk::prelude::*;
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default, Clone, Copy)]
struct Selection {
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    dragging: bool,
    finished: bool,
}

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
        window.set_exclusive_zone(-1);

        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);

        window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::OnDemand);

        let css = gtk::CssProvider::new();
        css.load_from_data("window { background-color: transparent; }");

        gtk::style_context_add_provider_for_display(
            &gtk::gdk::Display::default().unwrap(),
            &css,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let selection = Rc::new(RefCell::new(Selection::default()));
        let area = gtk::DrawingArea::new();

        area.set_hexpand(true);
        area.set_vexpand(true);

        let draw_selection = selection.clone();

        area.set_draw_func(move |_, cr, width, height| {
            let s = draw_selection.borrow();

            cr.set_operator(gtk::cairo::Operator::Over);
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.25);
            cr.paint().unwrap();

            if s.dragging || s.finished {
                let x = s.start_x.min(s.end_x);
                let y = s.start_y.min(s.end_y);
                let w = (s.end_x - s.start_x).abs();
                let h = (s.end_y - s.start_y).abs();

                cr.set_operator(gtk::cairo::Operator::Clear);
                cr.rectangle(x, y, w, h);
                cr.fill().unwrap();

                cr.set_operator(gtk::cairo::Operator::Over);

                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.set_line_width(2.0);
                cr.rectangle(x, y, w, h);
                cr.stroke().unwrap();
            }

            let _ = (width, height);
        });

        let drag = gtk::GestureDrag::new();

        let drag_selection = selection.clone();
        let drag_area = area.clone();

        drag.connect_drag_begin(move |_, x, y| {
            *drag_selection.borrow_mut() = Selection {
                start_x: x,
                start_y: y,
                end_x: x,
                end_y: y,
                dragging: true,
                finished: false,
            };

            drag_area.queue_draw();
        });

        let update_selection = selection.clone();
        let update_area = area.clone();

        drag.connect_drag_update(move |_, dx, dy| {
            let mut s = update_selection.borrow_mut();

            s.end_x = s.start_x + dx;
            s.end_y = s.start_y + dy;

            update_area.queue_draw();
        });

        let finish_selection = selection.clone();
        let finish_area = area.clone();

        drag.connect_drag_end(move |_, dx, dy| {
            let mut s = finish_selection.borrow_mut();

            s.end_x = s.start_x + dx;
            s.end_y = s.start_y + dy;
            s.dragging = false;
            s.finished = true;

            finish_area.queue_draw();

            println!(
                "Selected rectangle: x={}, y={}, width={}, height={}",
                s.start_x.min(s.end_x),
                s.start_y.min(s.end_y),
                (s.end_x - s.start_x).abs(),
                (s.end_y - s.start_y).abs()
            );
        });

        area.add_controller(drag);

        let keys = gtk::EventControllerKey::new();
        let close_window = window.clone();

        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                close_window.close();
                return gtk::glib::Propagation::Stop;
            }

            gtk::glib::Propagation::Proceed
        });

        window.add_controller(keys);
        window.set_child(Some(&area));
        window.fullscreen();
        window.present();
    });

    app.run();
}
