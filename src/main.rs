#![deny(warnings)]

#[macro_use]
extern crate log;

use glium::{glutin, Surface};
use std::time::{Duration, Instant};
use glium::glutin::event_loop::{EventLoop, ControlFlow};
use glium::glutin::event::{Event, StartCause};
use glium::glutin::dpi::PhysicalSize;
use std::collections::BTreeMap;

#[derive(Debug)]
enum Error {
    String(String),
}

type Result<T> = std::result::Result<T, Error>;

enum Action {
    Stop,
    Continue,
}

fn start_loop<F>(event_loop: EventLoop<()>, mut callback: F)->! where F: 'static + FnMut(&Vec<Event<()>>) -> Action {
    let mut events_buffer = Vec::new();
    let mut next_frame_time = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        let run_callback = match event.to_static() {
            Some(Event::NewEvents(cause)) => {
                match cause {
                    StartCause::ResumeTimeReached { .. } | StartCause::Init => {
                        true
                    },
                    _ => false
                }
            },
            Some(event) => {
                events_buffer.push(event);
                false
            }
            None => {
                // Ignore this event.
                false
            },
        };

        let action = if run_callback {
            let action = callback(&events_buffer);
            next_frame_time = Instant::now() + Duration::from_nanos(16666667);
            // TODO: Add back the old accumulator loop in some way

            events_buffer.clear();
            action
        } else {
            Action::Continue
        };

        match action {
            Action::Continue => {
                *control_flow = ControlFlow::WaitUntil(next_frame_time);
            },
            Action::Stop => *control_flow = ControlFlow::Exit
        }
    })
}

#[derive(Debug, Clone)]
struct Point {
    t: u32,

    #[allow(dead_code)] // WIP
    v: u16,
}

struct Store {
    last_t: u32,
    all: BTreeMap<u32, u16>,
}

impl Store {
    fn new() -> Store {
        Store {
            last_t: 0,
            all: BTreeMap::new(),
        }
    }

    fn ingest(&mut self, ps: &[Point]) -> Result<()> {
        for p in ps {
            if p.t <= self.last_t {
                return Err(Error::String("t <= last_t".to_owned()));
            }
            self.last_t = p.t;

            self.all.insert(p.t, p.v);
        }

        trace!("ingest all.len={} last_t={}", self.all.len(), self.last_t);

        Ok(())
    }

    fn purge(&mut self, t0: u32, t1: u32) -> Result<()> {
        for t in self.all.range(t0..t1).map(|(t,_v)| *t).collect::<Vec<u32>>() {
            self.all.remove(&t);
        }
        Ok(())
    }

    fn query(&self, t0: u32, t1: u32) -> Result<Vec<Point>> {
        let rv: Vec<Point> = self.all.range(t0..t1).map(|(t,v)| Point { t: *t, v: *v }).collect();
        Ok(rv)
    }

    fn last_t(&self) -> u32 {
        self.last_t
    }
}

const GEN_POINTS: u32 = 200;
const GEN_T_INTERVAL: u32 = 20;

struct TestDataGenerator {
    curr_t: u32,
}

impl TestDataGenerator {
    fn new() -> TestDataGenerator {
        TestDataGenerator {
            curr_t: 1
        }
    }

    fn gen_data(&mut self) -> Vec<Point> {
        let mut rv: Vec<Point> = Vec::with_capacity(GEN_POINTS as usize);
        for _i in 0..GEN_POINTS {
            rv.push(Point {
                t: self.curr_t,
                v: ((((self.curr_t as f32 / 10000.0).sin() + 1.0) / 2.0) * std::u16::MAX as f32) as u16,
            });
            self.curr_t += GEN_T_INTERVAL;
        }
        rv
    }
}

fn do_it(store: &Store,
         pb: &mut [u8], pbw: usize, pbh: usize,
         t0: u32, t1: u32, v0: u16, v1: u16
) -> Result<()> {

    trace!("do_it: pbw={}", pbw);
    let points = store.query(t0, t1)?;
    for p in points {
        assert!(p.t >= t0 && p.t <= t1);

        let x = (((p.t-t0) as f32 / (t1-t0) as f32) * pbw as f32) as usize;
        let y = (((p.v-v0) as f32 / (v1-v0) as f32) * pbh as f32) as usize;

        if !(x < pbw) {
            // Should be guaranteed by store.query.
            panic!("x < pbw: x={} pbw={}", x, pbw);
        }
        if y >= pbh {
            // Skip points that are outside our render patch.
            continue;
        }

        let i = 3*(pbw * y + x);
        pb[i] = 0u8;
        pb[i+1] = 255u8;
        pb[i+2] = 0u8;
    }

    Ok(())
}

const WIN_W: u16 = 800;
const WIN_H: u16 = 200;

// t per x pixel
const ZOOM_X: f32 = 500.0;

fn main() {
    env_logger::init();

    // Building the display, ie. the main object
    let event_loop = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(WIN_W, WIN_H));
    let cb = glutin::ContextBuilder::new().with_vsync(true);
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();

    let dest_texture = glium::Texture2d::empty_with_format(&display,
                                               glium::texture::UncompressedFloatFormat::U8U8U8U8,
                                               glium::texture::MipmapsOption::NoMipmap,
                                               WIN_W as u32, WIN_H as u32).unwrap();
    dest_texture.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);

    let mut g = TestDataGenerator::new();
    let mut s = Store::new();

    let mut fps_timer = Instant::now();
    let mut fps_count = 0;

    // the main loop
    start_loop(event_loop, move |events| {
        let t0 = s.last_t();
        s.ingest(&g.gen_data()).unwrap();
        let t1 = s.last_t();
        s.purge(0, t1 - (WIN_W as f32 * ZOOM_X) as u32).unwrap();

        let patch_dims = (((t1 - t0) as f32 / ZOOM_X) as usize, WIN_H as usize);
        let mut patch_bytes = vec![0u8; patch_dims.0 * patch_dims.1 * 3];
        do_it(&s, &mut patch_bytes, patch_dims.0, patch_dims.1,
              t0, t1, 0, std::u16::MAX).unwrap();
        let patch = glium::texture::RawImage2d::from_raw_rgb(patch_bytes, (patch_dims.0 as u32, patch_dims.1 as u32));
        let patch_texture = glium::Texture2d::new(&display, patch).unwrap();

        let dest_rect = glium::BlitTarget {
            left: ((t0 as f32 / ZOOM_X) as u32) % WIN_W as u32,
            bottom: 0u32,
            width: patch_dims.0 as i32,
            height: patch_dims.1 as i32,
        };

        trace!("dest_rect: {:?}", dest_rect);

        patch_texture.as_surface().blit_whole_color_to(
            &dest_texture.as_surface(), &dest_rect,
            glium::uniforms::MagnifySamplerFilter::Linear);

        // drawing a frame
        let target = display.draw();
        dest_texture.as_surface().fill(&target, glium::uniforms::MagnifySamplerFilter::Linear);
        target.finish().unwrap();

        fps_count += 1;
        if fps_timer.elapsed().as_secs() >= 1 {
            debug!("fps: {}", fps_count);
            fps_timer = Instant::now();
            fps_count = 0;
        }

        let mut action = Action::Continue;

            // handling the events received by the window since the last frame
        for event in events {
            match event {
                glutin::event::Event::WindowEvent { event, .. } => match event {
                    glutin::event::WindowEvent::CloseRequested => action = Action::Stop,
                    _ => (),
                },
                _ => (),
            }
        }

        action
    });
}
