#![deny(warnings)]

use rt_graph::{GraphWindow, GraphWindowBuilder, TestDataGenerator};
use std::sync::Mutex;


fn main() {
    env_logger::init();

    let w: GraphWindow =
        GraphWindowBuilder::default()
            .data_source(Mutex::new(Box::new(TestDataGenerator::new())))
            .build().unwrap();
    w.main().unwrap();
}
