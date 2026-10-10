use gtk::prelude::*;
use gtk4 as gtk;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

const BUTTON_RADIUS: f64 = 18.0;
const BUTTON_MARGIN: f64 = 8.0;

fn capture_screen() -> Result<(), Box<dyn Error>> {
    let status = Command::new("grim")
        .arg("/tmp/snap-background.png")
        .status()?;

    if !status.success() {
        return Err("Failed to capture screen".into());
    }

    Ok(())
}

#[derive(Default, Clone, Copy)]
struct Selection {
    start_x: f64,
    start_y: f64,
    end_x: f64,
    end_y: f64,
    dragging: bool,
    finished: bool,
    text_mode: bool,
}

impl Selection {
    fn rect(&self) -> (f64, f64, f64, f64) {
        (
            self.start_x.min(self.end_x),
            self.start_y.min(self.end_y),
            (self.end_x - self.start_x).abs(),
            (self.end_y - self.start_y).abs(),
        )
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Save,
    Text,
}

const BUTTONS: [Action; 2] = [Action::Save, Action::Text];

fn button_center(s: &Selection, index: usize) -> (f64, f64) {
    let (x, y, w, _) = s.rect();

    let mut cx = x + w - BUTTON_RADIUS;

    let cy = if y - 2.0 * BUTTON_RADIUS - BUTTON_MARGIN < 0.0 {
        cx -= BUTTON_MARGIN;
        y + BUTTON_RADIUS + BUTTON_MARGIN
    } else {
        y - BUTTON_RADIUS - BUTTON_MARGIN
    };

    cx -= index as f64 * (2.0 * BUTTON_RADIUS + BUTTON_MARGIN);

    (cx, cy)
}

fn draw_button_background(cr: &gtk::cairo::Context, cx: f64, cy: f64, active: bool) {
    cr.arc(cx, cy, BUTTON_RADIUS, 0.0, std::f64::consts::TAU);
    if active {
        cr.set_source_rgb(1.0, 1.0, 1.0);
    } else {
        cr.set_source_rgb(0.0, 0.0, 0.0);
    }
    let _ = cr.fill_preserve();

    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.set_line_width(2.0);
    let _ = cr.stroke();
}

fn draw_button(cr: &gtk::cairo::Context, action: Action, cx: f64, cy: f64, active: bool) {
    draw_button_background(cr, cx, cy, active);

    if active {
        cr.set_source_rgb(0.0, 0.0, 0.0);
    } else {
        cr.set_source_rgb(1.0, 1.0, 1.0);
    }
    cr.set_line_width(3.0);
    cr.set_line_cap(gtk::cairo::LineCap::Round);
    cr.set_line_join(gtk::cairo::LineJoin::Round);

    match action {
        Action::Save => {
            cr.move_to(cx - 7.0, cy + 0.5);
            cr.line_to(cx - 2.0, cy + 6.0);
            cr.line_to(cx + 7.5, cy - 6.0);
        }
        Action::Text => {
            cr.move_to(cx - 6.5, cy - 6.0);
            cr.line_to(cx + 6.5, cy - 6.0);
            cr.move_to(cx, cy - 6.0);
            cr.line_to(cx, cy + 7.0);
        }
    }
    let _ = cr.stroke();
}

fn draw_check_button(cr: &gtk::cairo::Context, cx: f64, cy: f64) {
    cr.arc(cx, cy, BUTTON_RADIUS, 0.0, std::f64::consts::TAU);
    cr.set_source_rgb(0.0, 0.0, 0.0);
    let _ = cr.fill_preserve();

    cr.set_source_rgb(1.0, 1.0, 1.0);
    cr.set_line_width(2.0);
    let _ = cr.stroke();

    cr.set_line_width(3.0);
    cr.set_line_cap(gtk::cairo::LineCap::Round);
    cr.set_line_join(gtk::cairo::LineJoin::Round);
    cr.move_to(cx - 7.0, cy + 0.5);
    cr.line_to(cx - 2.0, cy + 6.0);
    cr.line_to(cx + 7.5, cy - 6.0);
    let _ = cr.stroke();
}

fn save_selection(
    screenshot: &gtk::cairo::ImageSurface,
    s: &Selection,
    area_width: f64,
    area_height: f64,
) -> Result<PathBuf, Box<dyn Error>> {
    let scale_x = screenshot.width() as f64 / area_width;
    let scale_y = screenshot.height() as f64 / area_height;

    let (x, y, w, h) = s.rect();

    let px = (x * scale_x).round().max(0.0);
    let py = (y * scale_y).round().max(0.0);
    let pw = (w * scale_x)
        .round()
        .min(screenshot.width() as f64 - px)
        .max(1.0);
    let ph = (h * scale_y)
        .round()
        .min(screenshot.height() as f64 - py)
        .max(1.0);

    let out = gtk::cairo::ImageSurface::create(gtk::cairo::Format::ARgb32, pw as i32, ph as i32)?;

    {
        let cr = gtk::cairo::Context::new(&out)?;
        cr.set_source_surface(screenshot, -px, -py)?;
        cr.paint()?;
    }

    let home = std::env::var("HOME")?;
    let dir = PathBuf::from(home).join("Pictures/snap");
    std::fs::create_dir_all(&dir)?;

    let secs = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let path = dir.join(format!("snap-{secs}.png"));

    let mut file = File::create(&path)?;
    out.write_to_png(&mut file)?;

    Ok(path)
}

fn main() {
    if let Err(error) = capture_screen() {
        eprintln!("Screen capture failed: {error}");
        return;
    }

    let app = gtk::Application::builder()
        .application_id("com.timkuessner.snap")
        .build();

    app.connect_activate(|app| {
        let screenshot = match File::open("/tmp/snap-background.png") {
            Ok(mut file) => match gtk::cairo::ImageSurface::create_from_png(&mut file) {
                Ok(surface) => surface,
                Err(error) => {
                    eprintln!("Could not load screenshot: {error}");
                    return;
                }
            },
            Err(error) => {
                eprintln!("Could not open screenshot: {error}");
                return;
            }
        };

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
        let draw_shot = screenshot.clone();

        area.set_draw_func(move |_, cr, _width, _height| {
            let s = draw_selection.borrow();

            let _ = cr.set_source_surface(&draw_shot, 0.0, 0.0);
            let _ = cr.paint();

            cr.set_operator(gtk::cairo::Operator::Over);
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.45);
            let _ = cr.paint();

            if s.dragging || s.finished {
                let (x, y, w, h) = s.rect();

                cr.save().unwrap();
                cr.rectangle(x, y, w, h);
                cr.clip();

                let _ = cr.set_source_surface(&draw_shot, 0.0, 0.0);
                let _ = cr.paint();

                cr.restore().unwrap();

                cr.set_source_rgb(1.0, 1.0, 1.0);
                cr.set_line_width(2.0);
                cr.rectangle(x, y, w, h);
                let _ = cr.stroke();

                if s.finished && w > 0.0 && h > 0.0 {
                    for (i, action) in BUTTONS.iter().enumerate() {
                        let (cx, cy) = button_center(&s, i);
                        let active = *action == Action::Text && s.text_mode;
                        draw_button(cr, *action, cx, cy, active);
                    }
                }
            }
        });

        let drag = gtk::GestureDrag::new();

        let drag_selection = selection.clone();
        let drag_area = area.clone();
        let drag_window = window.clone();
        let drag_shot = screenshot.clone();

        drag.connect_drag_begin(move |_, x, y| {
            let area_w = drag_area.width() as f64;
            let area_h = drag_area.height() as f64;

            let hit = {
                let s = drag_selection.borrow();
                let (_, _, w, h) = s.rect();

                if s.finished && w > 0.0 && h > 0.0 {
                    BUTTONS
                        .iter()
                        .enumerate()
                        .find(|(i, _)| {
                            let (cx, cy) = button_center(&s, *i);
                            (x - cx).powi(2) + (y - cy).powi(2) <= BUTTON_RADIUS.powi(2)
                        })
                        .map(|(_, a)| *a)
                } else {
                    None
                }
            };

            match hit {
                Some(Action::Save) => {
                    let s = *drag_selection.borrow();
                    match save_selection(&drag_shot, &s, area_w, area_h) {
                        Ok(path) => println!("Saved to {}", path.display()),
                        Err(error) => eprintln!("Could not save selection: {error}"),
                    }
                    drag_window.close();
                    return;
                }
                Some(Action::Text) => {
                    {
                        let mut s = drag_selection.borrow_mut();
                        s.text_mode = !s.text_mode;
                    }
                    println!("Text mode toggled");
                    drag_area.queue_draw();
                    return;
                }
                None => {}
            }

            if drag_selection.borrow().text_mode {
                return;
            }

            *drag_selection.borrow_mut() = Selection {
                start_x: x,
                start_y: y,
                end_x: x,
                end_y: y,
                dragging: true,
                finished: false,
                text_mode: false,
            };

            drag_area.queue_draw();
        });

        let update_selection = selection.clone();
        let update_area = area.clone();

        drag.connect_drag_update(move |_, dx, dy| {
            let mut s = update_selection.borrow_mut();
            if !s.dragging {
                return;
            }

            s.end_x = s.start_x + dx;
            s.end_y = s.start_y + dy;

            update_area.queue_draw();
        });

        let finish_selection = selection.clone();
        let finish_area = area.clone();

        drag.connect_drag_end(move |_, dx, dy| {
            let mut s = finish_selection.borrow_mut();
            if !s.dragging {
                return;
            }

            s.end_x = s.start_x + dx;
            s.end_y = s.start_y + dy;
            s.dragging = false;
            s.finished = true;

            finish_area.queue_draw();

            let (x, y, w, h) = s.rect();
            println!("Selected rectangle: x={x}, y={y}, width={w}, height={h}");
        });

        area.add_controller(drag);

        let keys = gtk::EventControllerKey::new();
        let close_window = window.clone();
        let key_selection = selection.clone();
        let key_area = area.clone();

        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gtk::gdk::Key::Escape {
                let was_text_mode = {
                    let mut s = key_selection.borrow_mut();
                    let was = s.text_mode;
                    s.text_mode = false;
                    was
                };

                if was_text_mode {
                    key_area.queue_draw();
                } else {
                    close_window.close();
                }
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
