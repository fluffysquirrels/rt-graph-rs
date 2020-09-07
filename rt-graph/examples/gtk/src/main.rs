use gio::prelude::*;
use gtk::prelude::*;

use std::env::args;

fn main() {
    env_logger::init();
    let application =
        gtk::Application::new(Some("com.github.fluffysquirrels.rt-graph"), Default::default())
            .expect("Initialization failed...");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindowBuilder::new()
        .application(application)
        .title("rt-graph")
        .border_width(10)
        .window_position(gtk::WindowPosition::Center)
        .default_width(800)
        .default_height(300)
        .build();

    let win_box = gtk::BoxBuilder::new()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .build();
    window.add(&win_box);

    let graph = gtk::DrawingAreaBuilder::new()
        .height_request(200)
        .width_request(800)
        .build();
    win_box.add(&graph);
    graph.connect_draw(move |_ctrl, ctx| {
        ctx.rectangle(0.0, 0.0, 800.0, 200.0);
        ctx.set_source_rgb(0.8, 0.8, 0.8);
        ctx.fill();
        Inhibit(false)
    });

    let buttons_box = gtk::BoxBuilder::new()
        .orientation(gtk::Orientation::Horizontal)
        .height_request(50)
        .build();
    win_box.add(&buttons_box);

    let btn_pause = gtk::ButtonBuilder::new()
        .label("Pause")
        .build();
    buttons_box.add(&btn_pause);
    btn_pause.connect_clicked(move |b| b.set_label("Clicked"));

    let btn_zoom_x_in = gtk::ButtonBuilder::new()
        .label("Zoom X in")
        .build();
    buttons_box.add(&btn_zoom_x_in);
    btn_zoom_x_in.connect_clicked(move |b| b.set_label("Clicked"));

    let btn_zoom_x_out = gtk::ButtonBuilder::new()
        .label("Zoom X out")
        .build();
    buttons_box.add(&btn_zoom_x_out);
    btn_zoom_x_out.connect_clicked(move |b| b.set_label("Clicked"));

    window.show_all();
}
