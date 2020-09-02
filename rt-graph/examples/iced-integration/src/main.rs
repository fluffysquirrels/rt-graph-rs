mod controls;
mod tex_scene;

use controls::{Controls, Running};
use tex_scene::TexScene;

#[macro_use]
extern crate log;

use futures::task::SpawnExt;
use iced_wgpu::{wgpu, Backend, Renderer, Settings, Viewport};
use iced_winit::{conversion, futures, program, winit, Debug, Size};
use rt_graph::{Color, DataSource, Store, TestDataGenerator};
use std::time::{Duration, Instant};
use winit::{
    dpi::PhysicalPosition,
    event::{Event, ModifiersState, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

const GRAPH_W: u32 = 800;
const GRAPH_H: u32 = 200;
const WINDOW_H: u32 = 300;

const BASE_ZOOM_X: f32 = 1000.0;

const BYTES_PER_PIXEL: usize = 4;

pub fn main() {
    env_logger::init();

    // Initialize winit
    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_inner_size(winit::dpi::PhysicalSize::new(GRAPH_W, WINDOW_H))
        .build(&event_loop).unwrap();

    let physical_size = window.inner_size();
    let mut viewport = Viewport::with_physical_size(
        Size::new(physical_size.width, physical_size.height),
        window.scale_factor(),
    );
    let mut cursor_position = PhysicalPosition::new(-1.0, -1.0);
    let mut modifiers = ModifiersState::default();

    // Initialize wgpu
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    let (mut device, queue) = futures::executor::block_on(async {
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::Default,
                compatible_surface: Some(&surface),
            },
        )
        .await
        .expect("Request adapter");

        adapter
            .request_device(&wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                shader_validation: false,
            }, None)
            .await
            .expect("Request device")
    });

    let swap_chain_format = wgpu::TextureFormat::Bgra8UnormSrgb;

    let mut swap_chain = {
        let size = window.inner_size();

        device.create_swap_chain(
            &surface,
            &wgpu::SwapChainDescriptor {
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                format: swap_chain_format,
                width: size.width,
                height: size.height,
                present_mode: wgpu::PresentMode::Mailbox,
            },
        )
    };
    let mut resized = false;

    let mut staging_belt = wgpu::util::StagingBelt::new(5 * 1024);
    let mut local_pool = futures::executor::LocalPool::new();

    // Initialize scene and GUI controls
    let mut tex_scene = TexScene::init(
        GRAPH_W, GRAPH_H, swap_chain_format,
        &device,
        &queue,
    );
    let controls = Controls::new();

    // Initialize iced
    let mut debug = Debug::new();
    let mut renderer =
        Renderer::new(Backend::new(&mut device, Settings::default()));

    let mut state = program::State::new(
        controls,
        viewport.logical_size(),
        conversion::cursor_position(cursor_position, viewport.scale_factor()),
        &mut renderer,
        &mut debug,
        );

    // Initialize the backing texture
    let backing_tex = device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d { width: GRAPH_W, height: WINDOW_H, depth: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsage::COPY_SRC |
               wgpu::TextureUsage::COPY_DST |
               wgpu::TextureUsage::SAMPLED,
    });
    clear_backing_tex(&queue, &backing_tex);

    let mut fps_timer = Instant::now();
    let mut fps_count = 0;

    let mut next_ingest_timer = Instant::now();
    let mut data_source = TestDataGenerator::new();
    let mut store = Store::new(data_source.get_num_values().unwrap() as u8);

    let mut last_t_drawn = 0;
    let mut last_x_drawn = 0;

    let mut zoom_x_drawn = state.program().zoom_x();

    // Run event loop
    event_loop.run(move |event, _, control_flow| {
        let begin_frame = Instant::now();
        let next_frame = begin_frame + Duration::from_nanos(16_666_667);
        // *control_flow = ControlFlow::Poll;
        *control_flow = ControlFlow::WaitUntil(next_frame);

        trace!("event:{:?}", event);
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    WindowEvent::CursorMoved { position, .. } => {
                        cursor_position = position;
                    }
                    WindowEvent::ModifiersChanged(new_modifiers) => {
                        modifiers = new_modifiers;
                    }
                    WindowEvent::Resized(new_size) => {
                        viewport = Viewport::with_physical_size(
                            Size::new(new_size.width, new_size.height),
                            window.scale_factor(),
                        );

                        resized = true;
                    }
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }

                // Map window event to iced event
                if let Some(event) = iced_winit::conversion::window_event(
                    &event,
                    window.scale_factor(),
                    modifiers,
                ) {
                    state.queue_event(event);
                }
            }
            Event::MainEventsCleared => {
                // If there are events pending
                if !state.is_queue_empty() {
                    // We update iced
                    let _ = state.update(
                        viewport.logical_size(),
                        conversion::cursor_position(
                            cursor_position,
                            viewport.scale_factor(),
                        ),
                        None,
                        &mut renderer,
                        &mut debug,
                    );

                    // and request a redraw
                    window.request_redraw();
                }
            }
            Event::RedrawRequested(_) |
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                if resized {
                    let size = window.inner_size();

                    swap_chain = device.create_swap_chain(
                        &surface,
                        &wgpu::SwapChainDescriptor {
                            usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                            format: swap_chain_format,
                            width: size.width,
                            height: size.height,
                            present_mode: wgpu::PresentMode::Mailbox,
                        },
                        );

                    resized = false;
                }

                let frame = swap_chain.get_current_frame().expect("Next frame");
                let frame_tex_view = &frame.output.view;

                // Render the backing texture to the frame.
                tex_scene.render(frame_tex_view, &device, &queue,
                                 &backing_tex, GRAPH_W, GRAPH_H);

                let mut encoder = device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor { label: None },
                    );

                // And then iced on top
                let mouse_interaction = renderer.backend_mut().draw(
                    &mut device,
                    &mut staging_belt,
                    &mut encoder,
                    frame_tex_view,
                    &viewport,
                    state.primitive(),
                    &debug.overlay(),
                    );

                // Then we submit the work
                staging_belt.finish();
                queue.submit(Some(encoder.finish()));

                // And update the mouse cursor
                window.set_cursor_icon(
                    iced_winit::conversion::mouse_interaction(
                        mouse_interaction,
                        ),
                    );

                // Recall the staging belt
                local_pool
                    .spawner()
                    .spawn(staging_belt.recall())
                    .expect("Recall staging buffers");
                local_pool.run_until_stalled();

                // Calculate FPS and log it once per second.
                fps_count += 1;
                if fps_timer.elapsed().as_secs() >= 1 {
                    debug!("fps: {}", fps_count);
                    fps_timer = Instant::now();
                    fps_count = 0;
                }
            },
            e => {
                trace!("Unhandled event: {:?}", e);
            }
        }

        let zoom_x_latest = state.program().zoom_x();
        if zoom_x_drawn != zoom_x_latest {
            zoom_x_drawn = zoom_x_latest;
            let t_latest = store.last_t();

            let cols = data_source.get_colors().unwrap();
            let patch_offset_x = 0;
            let window_dt = (GRAPH_W as f32 * zoom_x_drawn) as u32;
            clear_backing_tex(&queue, &backing_tex);
            if t_latest < window_dt {
                // Values don't fill the graph at the new zoom level,
                // render what we have on the left
                let t0 = 0;
                let t1 = t_latest;
                let w = (t1 as f32 / zoom_x_drawn).floor() as u32;
                let patch_dims = (w as usize,
                                  GRAPH_H as usize);
                let mut patch_bytes = vec![0u8; patch_dims.0 * patch_dims.1 * BYTES_PER_PIXEL];
                render_patch(&store, &cols, &mut patch_bytes, patch_dims.0, patch_dims.1,
                             t0, t1, 0, std::u16::MAX).unwrap();
                last_x_drawn = w;

                copy_patch(&queue, &backing_tex,
                           &patch_bytes[..], patch_dims.0 as u32, patch_dims.1 as u32,
                           0, 0);
            } else {
                // t_latest >= window_dt
                // Values fill the graph at the new zoom level, so
                // render the latest data across the whole screen.

                let t0 = t_latest - (GRAPH_W as f32 * zoom_x_drawn) as u32;
                let t1 = t_latest;
                let patch_dims = (GRAPH_W as usize,
                                  GRAPH_H as usize);
                let mut patch_bytes = vec![0u8; patch_dims.0 * patch_dims.1 * BYTES_PER_PIXEL];
                render_patch(&store, &cols, &mut patch_bytes, patch_dims.0, patch_dims.1,
                             t0, t1, 0, std::u16::MAX).unwrap();
                let patch_offset_x = 0;
                last_x_drawn = 0;

                copy_patch(&queue, &backing_tex,
                           &patch_bytes[..], patch_dims.0 as u32, patch_dims.1 as u32,
                           0, 0);
            }


            last_t_drawn = t_latest;
        }

        while next_ingest_timer < Instant::now() {
            if state.program().running() == Running::Pause {
                next_ingest_timer = Instant::now();
                break;
            }

            store.ingest(&*data_source.get_data().unwrap()).unwrap();
            next_ingest_timer += Duration::from_nanos(16_666_667);
            let t_latest = store.last_t();

            // Discard old data if there is any
            let window_base_dt = (GRAPH_W as f32 * BASE_ZOOM_X) as u32;
            if t_latest >= window_base_dt {
                store.discard(0, t_latest - window_base_dt).unwrap();
            }

            // Calculate the size of the latest patch to render.
            let patch_dims = (((t_latest - last_t_drawn) as f32 / zoom_x_drawn).floor() as usize,
                              GRAPH_H as usize);
            // If there is more than a pixel's worth of data to render since we last drew,
            // then draw it.
            if patch_dims.0 >= 1 {
                let mut patch_bytes = vec![0u8; patch_dims.0 * patch_dims.1 * BYTES_PER_PIXEL];
                let new_t = last_t_drawn + (patch_dims.0 as f32 * zoom_x_drawn) as u32;
                let cols = data_source.get_colors().unwrap();
                render_patch(&store, &cols, &mut patch_bytes, patch_dims.0, patch_dims.1,
                             last_t_drawn, new_t, 0, std::u16::MAX).unwrap();

                let patch_offset_x = last_x_drawn;

                // TODO: For writes that overlap the right side of the texture
                // and wrap around, don't just ignore them but write a few pixels
                // on the right and a few on the left.
                if (patch_offset_x + (patch_dims.0 as u32)) < GRAPH_W {
                    copy_patch(&queue, &backing_tex,
                               &patch_bytes[..], patch_dims.0 as u32, patch_dims.1 as u32,
                               patch_offset_x, 0);
                }

                last_t_drawn = new_t;
                last_x_drawn = (last_x_drawn + patch_dims.0 as u32) % GRAPH_W as u32;
            }
        }

//         let now = Instant::now();
//         if now < next_frame {
//             std::thread::sleep(next_frame - now)
//         }
    })
}

fn clear_backing_tex(queue: &wgpu::Queue, tex: &wgpu::Texture) {
    queue.write_texture(
        wgpu::TextureCopyViewBase::<&wgpu::Texture> {
            texture: &tex,
            mip_level: 0,
            origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
        },
        &*vec![200u8; (GRAPH_W * WINDOW_H * 4) as usize],
        wgpu::TextureDataLayout {
            offset: 0,
            bytes_per_row: GRAPH_W * 4,
            rows_per_image: WINDOW_H,
        },
        wgpu::Extent3d {
            width: GRAPH_W,
            height: WINDOW_H,
            depth: 1,
        }
    );
}

fn render_patch(
    store: &Store, cols: &[Color],
    pb: &mut [u8], pbw: usize, pbh: usize,
    t0: u32, t1: u32, v0: u16, v1: u16
) -> Result<(), ()> {

    trace!("render_patch: pbw={}", pbw);
    assert!(pbw >= 1);
    let points = store.query(t0, t1).unwrap();
    for p in points {
        assert!(p.t >= t0 && p.t <= t1);

        let x = (((p.t-t0) as f32 / (t1-t0) as f32) * pbw as f32) as usize;
        if !(x < pbw) {
            // Should be guaranteed by store.query.
            panic!("x < pbw: x={} pbw={}", x, pbw);
        }

        for ch in 0..store.val_len() {
            let col = cols[ch as usize % cols.len()];
            let y = (((p.vals()[ch as usize]-v0) as f32 / (v1-v0) as f32) * pbh as f32) as usize;
            if y >= pbh {
                // Skip points that are outside our render patch.
                continue;
            }

            let i = 4*(pbw * y + x);
            pb[i]   = col.0; // R
            pb[i+1] = col.1; // G
            pb[i+2] = col.2; // B
            pb[i+3] = 255;   // A
        }
    }

    Ok(())
}

/// Write the new patch to the backing texture.
fn copy_patch(queue: &wgpu::Queue, backing_tex: &wgpu::Texture,
              data: &[u8], w: u32, h: u32,
              offset_x: u32, offset_y: u32) {
    queue.write_texture(
        wgpu::TextureCopyViewBase::<&wgpu::Texture> {
            texture: &backing_tex,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: offset_x,
                y: WINDOW_H - GRAPH_H + offset_y,
                z: 0
            },
        },
        data,
        wgpu::TextureDataLayout {
            offset: 0,
            bytes_per_row: w * (BYTES_PER_PIXEL as u32),
            rows_per_image: h,
        },
        wgpu::Extent3d {
            width: w,
            height: GRAPH_H,
            depth: 1,
        }
    );
}
