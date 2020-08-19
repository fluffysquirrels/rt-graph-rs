use crate::{DataSource, Point, Result};

const GEN_POINTS: u32 = 200;
const GEN_T_INTERVAL: u32 = 20;

#[derive(Debug)]
pub struct TestDataGenerator {
    curr_t: u32,
}

impl TestDataGenerator {
    pub fn new() -> TestDataGenerator {
        TestDataGenerator {
            curr_t: 1
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
                vs: vec![trig_sample(1.0/10000.0, 0.0, t),
                         trig_sample(1.0/10000.0, std::f32::consts::PI / 3.0, t),
                         trig_sample(1.0/5000.0,  0.0, t)],
            });
            self.curr_t += GEN_T_INTERVAL;
        }
        Ok(rv)
    }

    fn get_num_values(&self) -> Result<usize> {
        Ok(3)
    }
}

fn trig_sample(scale: f32, offset: f32, t: u32) -> u16 {
    ((((offset + t as f32 * scale).sin() + 1.0) / 2.0) * std::u16::MAX as f32) as u16
}
