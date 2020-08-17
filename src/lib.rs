#![deny(warnings)]

#[macro_use]
extern crate derive_builder;

#[macro_use]
extern crate log;

use std::fmt::Debug;

mod graph_window;
pub use graph_window::{GraphWindow, GraphWindowBuilder};

mod store;
use store::Store;

mod test_data_generator;
pub use test_data_generator::TestDataGenerator;

#[derive(Debug)]
pub enum Error {
    String(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Point {
    t: u32,
    vs: Vec<u16>,
}

impl Point {
    pub fn vals(&self) -> &[u16] {
        &self.vs
    }
}

pub trait DataSource: Debug + Send + Sync {
    fn get_data(&mut self) -> Result<Vec<Point>>;
}
