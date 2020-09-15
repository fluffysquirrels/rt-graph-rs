#![deny(warnings)]

#[macro_use]
extern crate derive_builder;

#[macro_use]
extern crate log;

use std::fmt::Debug;

mod graph;
pub use graph::{Config, ConfigBuilder, Graph};

mod signal;
pub use signal::Signal;

mod store;
pub use store::Store;

mod test_data_generator;
pub use test_data_generator::TestDataGenerator;

#[derive(Debug)]
pub enum Error {
    String(String),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Point {
    pub t: u32,
    pub vs: Vec<u16>,
}

impl Point {
    pub fn vals(&self) -> &[u16] {
        &self.vs
    }
}

#[derive(Clone, Copy)]
pub struct Color(pub u8, pub u8, pub u8);

pub trait DataSource: Debug + Send {
    fn get_data(&mut self) -> Result<Vec<Point>>;
    fn get_num_values(&self) -> Result<usize>;
    fn get_colors(&self) -> Result<Vec<Color>> {
        Ok(vec![Color(255u8, 0u8,   0u8),
                Color(0u8,   255u8, 0u8),
                Color(0u8,   0u8,   255u8)
        ])
    }
}
