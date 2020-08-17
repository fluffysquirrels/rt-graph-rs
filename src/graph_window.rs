use glium::{glutin, Surface};
use std::time::{Duration, Instant};
use glium::glutin::event_loop::{EventLoop, ControlFlow};
use glium::glutin::event::{Event, StartCause};
use glium::glutin::dpi::PhysicalSize;

use crate::{Result, Store, TestDataGenerator};

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
            // next_frame_time = Instant::now() + Duration::from_nanos(16666667);
            next_frame_time = next_frame_time + Duration::from_nanos(16666667);
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

const COLS: [(u8, u8, u8); 3]
    = [(255u8, 0u8,   0u8),
       (0u8,   255u8, 0u8),
       (0u8,   0u8,   255u8)];

fn render_patch(store: &Store,
         pb: &mut [u8], pbw: usize, pbh: usize,
         t0: u32, t1: u32, v0: u16, v1: u16
) -> Result<()> {

    trace!("render_patch: pbw={}", pbw);
    let points = store.query(t0, t1)?;
    for p in points {
        assert!(p.t >= t0 && p.t <= t1);

        let x = (((p.t-t0) as f32 / (t1-t0) as f32) * pbw as f32) as usize;
        if !(x < pbw) {
            // Should be guaranteed by store.query.
            panic!("x < pbw: x={} pbw={}", x, pbw);
        }

        for ch in 0..store.val_len() {
            let col = COLS[ch as usize];
            let y = (((p.vals()[ch as usize]-v0) as f32 / (v1-v0) as f32) * pbh as f32) as usize;
            if y >= pbh {
                // Skip points that are outside our render patch.
                continue;
            }

            let i = 3*(pbw * y + x);
            pb[i]   = col.0;
            pb[i+1] = col.1;
            pb[i+2] = col.2;
        }
    }

    Ok(())
}

const WIN_W: u16 = 800;
const WIN_H: u16 = 200;

use once_cell::sync::OnceCell;

static TGW: OnceCell<GraphWindow> = OnceCell::new();

#[derive(Builder, Debug)]
pub struct GraphWindow {
    /// t per x pixel
    #[builder(default = "1000.0")]
    zoom_x: f32,
}

impl GraphWindow {
    pub fn main(self) -> Result<()> {
        TGW.set(self).expect("Not to have already set TGW, i.e. run main()");
        GraphWindow::main2(TGW.get().unwrap())
    }

    fn main2(w: &'static GraphWindow) -> Result<()> {
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
        let mut s = Store::new(3);

        let mut fps_timer = Instant::now();
        let mut fps_count = 0;

        // the main loop
        start_loop(event_loop, move |events| {
            let t0 = s.last_t();
            s.ingest(&g.gen_data()).unwrap();
            let t1 = s.last_t();
            s.discard(0, t1 - (WIN_W as f32 * w.zoom_x) as u32).unwrap();

            let patch_dims = (((t1 - t0) as f32 / w.zoom_x) as usize, WIN_H as usize);
            let mut patch_bytes = vec![0u8; patch_dims.0 * patch_dims.1 * 3];
            render_patch(&s, &mut patch_bytes, patch_dims.0, patch_dims.1,
                         t0, t1, 0, std::u16::MAX).unwrap();
            let patch = glium::texture::RawImage2d::from_raw_rgb(patch_bytes, (patch_dims.0 as u32, patch_dims.1 as u32));
            let patch_texture = glium::Texture2d::new(&display, patch).unwrap();

            let dest_rect = glium::BlitTarget {
                left: ((t0 as f32 / w.zoom_x) as u32) % WIN_W as u32,
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
}
