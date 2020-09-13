#![deny(warnings)]
#![allow(deprecated)]

use rt_graph::{GraphWindow, GraphWindowBuilder, TestDataGenerator};

fn main() {
    env_logger::init();

    let w: GraphWindow =
        GraphWindowBuilder::default()
            .data_source(TestDataGenerator::new())
            .build().unwrap();
    w.main().unwrap();
}
