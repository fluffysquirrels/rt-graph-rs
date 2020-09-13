// #[macro_use]
// extern crate log;

use gio::prelude::*;
use gtk::prelude::*;
use rt_graph::{ConfigBuilder, Graph, TestDataGenerator};
use std::{
    env::args,
};

fn main() {
    env_logger::init();
    let application =
        gtk::Application::new(Some("com.github.fluffysquirrels.rt-graph.gtk-example"),
                              gio::ApplicationFlags::default())
            .expect("Application::new failed");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindowBuilder::new()
        .application(application)
        .title("rt-graph")
        .border_width(8)
        .window_position(gtk::WindowPosition::Center)
        .build();

    let config = ConfigBuilder::default()
        .data_source(TestDataGenerator::new())
        .build()
        .unwrap();
    let g = Graph::build_ui(config, &window);

    window.show_all();
}
