use crate::{Error, Point, Result};
use std::collections::BTreeMap;

pub struct Store {
    last_t: u32,
    val_len: u8,
    all: BTreeMap<u32, Vec<u16>>,
}

impl Store {
    pub fn new(val_len: u8) -> Store {
        Store {
            last_t: 0,
            val_len,
            all: BTreeMap::new(),
        }
    }

    pub fn ingest(&mut self, ps: &[Point]) -> Result<()> {
        for p in ps {
            if p.t <= self.last_t {
                return Err(Error::String("t <= last_t".to_owned()));
            }
            self.last_t = p.t;

            assert!(p.vs.len() == self.val_len as usize);
            self.all.insert(p.t, p.vs.clone());
        }

        trace!("ingest all.len={} last_t={}", self.all.len(), self.last_t);

        Ok(())
    }

    pub fn discard(&mut self, t0: u32, t1: u32) -> Result<()> {
        for t in self.all.range(t0..t1).map(|(t,_vs)| *t).collect::<Vec<u32>>() {
            self.all.remove(&t);
        }
        Ok(())
    }

    /// Returns a Vec of the points with t >= t0, < t1.
    pub fn query_range(&self, t0: u32, t1: u32) -> Result<Vec<Point>> {
        let rv: Vec<Point> =
            self.all.range(t0..t1)
                .map(|(t,vs)| Point { t: *t, vs: vs.clone() })
                .collect();
        trace!("query t0={} t1={} rv.len={}", t0, t1, rv.len());
        Ok(rv)
    }

    /// Returns the first point with t >= given t.
    pub fn query_point(&self, t: u32) -> Result<Option<Point>> {
        let rv = self.all.range(t..)
                     .map(|(t,vs)| Point { t: *t, vs: vs.clone() })
                     .next();
        Ok(rv)
    }

    pub fn last_t(&self) -> u32 {
        self.last_t
    }

    pub fn first_t(&self) -> u32 {
        self.query_point(0).unwrap()
                           .map(|pt| pt.t)
                           .unwrap_or(0)
    }

    pub fn val_len(&self) -> u8 {
        self.val_len
    }
}
