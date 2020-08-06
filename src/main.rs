#![deny(warnings)]

use glium::{glutin, Surface};

use std::time::{Duration, Instant};
use glium::glutin::event_loop::{EventLoop, ControlFlow};
use glium::glutin::event::{Event, StartCause};

pub enum Action {
    Stop,
    Continue,
}

pub fn start_loop<F>(event_loop: EventLoop<()>, mut callback: F)->! where F: 'static + FnMut(&Vec<Event<()>>) -> Action {
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

fn main() {
    // Building the display, ie. the main object
    let event_loop = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new();
    let cb = glutin::ContextBuilder::new().with_vsync(true);
    let display = glium::Display::new(wb, cb, &event_loop).unwrap();

    // building a texture with "OpenGL" drawn on it
    let patch_bytes = vec![100u8; 100 * 100 * 3];
    let patch_dimensions = (100,100);
    let patch = glium::texture::RawImage2d::from_raw_rgb(patch_bytes, patch_dimensions);
    let patch_texture = glium::Texture2d::new(&display, patch).unwrap();

    // building a 1024x1024 empty texture
    let dest_texture = glium::Texture2d::empty_with_format(&display,
                                               glium::texture::UncompressedFloatFormat::U8U8U8U8,
                                               glium::texture::MipmapsOption::NoMipmap,
                                               1024, 1024).unwrap();
    dest_texture.as_surface().clear_color(0.0, 0.0, 0.0, 1.0);

    // the main loop
    start_loop(event_loop, move |events| {
        // we have one out of 60 chances to blit one `opengl_texture` over `dest_texture`
        if rand::random::<f64>() <= 0.016666 {
            let (left, bottom, dimensions): (f32, f32, f32) = rand::random();
            let dest_rect = glium::BlitTarget {
                left: (left * dest_texture.get_width() as f32) as u32,
                bottom: (bottom * dest_texture.get_height().unwrap() as f32) as u32,
                width: (dimensions * dest_texture.get_width() as f32) as i32,
                height: (dimensions * dest_texture.get_height().unwrap() as f32) as i32,
            };

            patch_texture.as_surface().blit_whole_color_to(&dest_texture.as_surface(), &dest_rect,
                                                            glium::uniforms::MagnifySamplerFilter::Linear);
        }

        // drawing a frame
        let target = display.draw();
        dest_texture.as_surface().fill(&target, glium::uniforms::MagnifySamplerFilter::Linear);
        target.finish().unwrap();

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
