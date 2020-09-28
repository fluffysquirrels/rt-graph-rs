use crate::{DataSource, Point, Result};

/// A `DataSource` that returns no data points.
#[derive(Debug)]
pub struct NullDataSource;

impl DataSource for NullDataSource {
    fn get_data(&mut self) -> Result<Vec<Point>> {
        Ok(vec![])
    }

    fn get_num_values(&self) -> Result<usize> {
        Ok(1)
    }
}

impl NullDataSource {
    /// Constructs a new instance of `NullDataSource`.
    pub fn new() -> NullDataSource {
        NullDataSource
    }
}
