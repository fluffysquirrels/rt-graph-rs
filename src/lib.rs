#![deny(warnings)]

#[macro_use]
extern crate derive_builder;

#[macro_use]
extern crate log;

use std::fmt::Debug;

mod graph;
pub use graph::{Config, ConfigBuilder, Graph, PointStyle, View, ViewMode};

mod graph_with_controls;
pub use graph_with_controls::GraphWithControls;

pub mod observable_value;

mod signal;
pub use signal::Signal;

mod store;
use store::Store;

mod test_data_generator;
pub use test_data_generator::TestDataGenerator;

#[derive(Debug)]
pub enum Error {
    String(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A data point on a graph.
#[derive(Debug, Clone)]
pub struct Point {
    /// The time when this data point was emitted.
    pub t: u32,

    /// The values this point holds.
    pub vs: Vec<u16>,
}

impl Point {
    /// Return the values that this point holds.
    pub fn vals(&self) -> &[u16] {
        &self.vs
    }
}

#[derive(Clone, Copy)]
pub struct Color(pub u8, pub u8, pub u8);

/// Implement this to get your own data into a `Graph`.
pub trait DataSource: Debug + Send {
    /// Return whatever points you have available when this method is called.
    ///
    /// Each point must have a `t` field greater than the previous point.
    ///
    /// Each point must have a `vs` field with length equal to the
    /// value returned by `get_num_values`.
    ///
    /// This is currently called once a frame.
    fn get_data(&mut self) -> Result<Vec<Point>>;

    /// The number of values that each Point will have.
    fn get_num_values(&self) -> Result<usize>;

    /// Return the colors you want to use to display each value of the graph.
    ///
    /// Some sample colors are returned by default.
    ///
    /// If you don't supply enough colors for the number of values
    /// returned, these colors will be repeated.
    fn get_colors(&self) -> Result<Vec<Color>> {
        Ok(vec![Color(255u8, 0u8,   0u8),
                Color(0u8,   255u8, 0u8),
                Color(0u8,   0u8,   255u8)
        ])
    }
}
