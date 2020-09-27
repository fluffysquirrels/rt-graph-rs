use crate::{DataSource, Point, Result};

const GEN_POINTS: u32 = 200;
const GEN_T_INTERVAL: u32 = 20;

/// A struct that implements `DataSource` by showing dummy test data.
#[derive(Debug)]
pub struct TestDataGenerator {
    curr_t: u32,
    interval: u32,
    interval_inc: bool,
}

impl TestDataGenerator {
    /// Construct a new instance.
    pub fn new() -> TestDataGenerator {
        TestDataGenerator {
            curr_t: 1,
            interval: GEN_T_INTERVAL,
            interval_inc: false,
        }
    }
}

impl DataSource for TestDataGenerator {
    fn get_data(&mut self) -> Result<Vec<Point>> {
        let mut rv: Vec<Point> = Vec::with_capacity(GEN_POINTS as usize);
        for _i in 0..GEN_POINTS {
            let t = self.curr_t;
            rv.push(Point {
                t,
                vs: vec![trig_sample(1.0, 1.0/10000.0, 0.0, t),
                         ((100000.0 / (t as f64)) * trig_sample(1.0, 1.0/10000.0, std::f32::consts::PI / 3.0, t) as f64) as u16,
                         trig_sample(0.5, 1.0/5000.0,  0.0, t)],
            });

            self.curr_t += self.interval;
        }

        let switch = if self.interval_inc {
            self.interval += 1;
            self.interval == GEN_T_INTERVAL
        } else {
            self.interval -= 1;
            self.interval == 1
        };
        if switch {
            self.interval_inc = !self.interval_inc;
        }
        Ok(rv)
    }

    fn get_num_values(&self) -> Result<usize> {
        Ok(3)
    }
}

fn trig_sample(scale: f32, scale_period: f32, offset: f32, t: u32) -> u16 {
    let float_val = (offset + t as f32 * scale_period).sin() * scale;
    let int_val = (((float_val + 1.0) / 2.0) * std::u16::MAX as f32) as u16;
    int_val
}
