#![deny(warnings)]

#[macro_use]
extern crate derive_builder;

#[macro_use]
extern crate log;

mod graph_window;
use graph_window::{GraphWindow, GraphWindowBuilder};

#[derive(Debug)]
pub enum Error {
    String(String),
}

pub type Result<T> = std::result::Result<T, Error>;

fn main() {
    env_logger::init();

    let w: GraphWindow = GraphWindowBuilder::default().build().unwrap();
    w.main().unwrap();
}
