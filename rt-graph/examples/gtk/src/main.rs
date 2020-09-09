#[macro_use]
extern crate log;

use gdk::prelude::*;
use gio::prelude::*;
use glib::source::Continue;
use gtk::prelude::*;
use rt_graph::DataSource;
use std::{
    cell::{Cell, RefCell},
    env::args,
    rc::Rc,
    time::Instant,
};

const GRAPH_W: u32 = 800;
const GRAPH_H: u32 = 200;
const BASE_ZOOM_X: f32 = 1000.0;
const BYTES_PER_PIXEL: usize = 4;

struct WindowState {
    backing_surface: RefCell<cairo::Surface>,
    temp_surface: RefCell<cairo::Surface>,

    data_source: RefCell<Box<dyn rt_graph::DataSource>>,
    graph_drawing_area: gtk::DrawingArea,
    store: RefCell<rt_graph::Store>,

    last_t_drawn: Cell<u32>,
    zoom_x: Cell<f32>,

    fps_count: Cell<u16>,
    fps_timer: Cell<Instant>,
}

fn main() {
    env_logger::init();
    let application =
        gtk::Application::new(Some("com.github.fluffysquirrels.rt-graph"), Default::default())
            .expect("Application::new failed");

    application.connect_activate(|app| {
        build_ui(app);
    });

    application.run(&args().collect::<Vec<_>>());
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindowBuilder::new()
        .application(application)
        .title("rt-graph")
        .border_width(8)
        .window_position(gtk::WindowPosition::Center)
        .default_width(GRAPH_W as i32)
        .default_height((GRAPH_H + 100) as i32)
        .build();

    let win_box = gtk::BoxBuilder::new()
        .orientation(gtk::Orientation::Vertical)
        .spacing(8)
        .build();
    window.add(&win_box);

    let graph = gtk::DrawingAreaBuilder::new()
        .height_request(GRAPH_H as i32)
        .width_request(GRAPH_W as i32)
        .build();
    win_box.add(&graph);

    let buttons_box = gtk::BoxBuilder::new()
        .orientation(gtk::Orientation::Horizontal)
        .height_request(50)
        .build();
    win_box.add(&buttons_box);

    let btn_pause = gtk::ButtonBuilder::new()
        .label("Pause")
        .build();
    buttons_box.add(&btn_pause);
    btn_pause.connect_clicked(move |b| b.set_label("Clicked"));

    let btn_zoom_x_in = gtk::ButtonBuilder::new()
        .label("Zoom X in")
        .build();
    buttons_box.add(&btn_zoom_x_in);
    btn_zoom_x_in.connect_clicked(move |b| b.set_label("Clicked"));

    let btn_zoom_x_out = gtk::ButtonBuilder::new()
        .label("Zoom X out")
        .build();
    buttons_box.add(&btn_zoom_x_out);
    btn_zoom_x_out.connect_clicked(move |b| b.set_label("Clicked"));

    // Initialise WindowState

    // Show window here so we can get an instance of gdk::Window with
    // get_window() below, in order to create_similar_image_surface.
    window.show();

    let backing_surface = create_backing_surface(&window.get_window().unwrap(),
                                                 GRAPH_W, GRAPH_H);
    let temp_surface = create_backing_surface(&window.get_window().unwrap(),
                                              GRAPH_W, GRAPH_H);
    let ds = rt_graph::TestDataGenerator::new();
    let s = rt_graph::Store::new(ds.get_num_values().unwrap() as u8);
    let ws = Rc::new(WindowState {
        backing_surface: RefCell::new(backing_surface),
        temp_surface: RefCell::new(temp_surface),

        store: RefCell::new(s),
        data_source: RefCell::new(Box::new(ds)),
        graph_drawing_area: graph.clone(),

        last_t_drawn: Cell::new(0),
        zoom_x: Cell::new(1000.0),

        fps_count: Cell::new(0),
        fps_timer: Cell::new(Instant::now()),
    });

    // Set signal handlers that require WindowState
    let wsc = ws.clone();
    graph.connect_draw(move |ctrl, ctx| {
        graph_draw(ctrl, ctx, &*wsc)
    });

    let wsc = ws.clone();
    let _tick_id = glib::source::timeout_add_local(16 /* ms */, move || {
        tick(&*wsc);
        Continue(true)
    });

    // Show everything recursively
    window.show_all();
}

fn graph_draw(_ctrl: &gtk::DrawingArea, ctx: &cairo::Context, ws: &WindowState) -> Inhibit {
    trace!("graph_draw");

    // Copy from the backing_surface, which was updated elsewhere
    ctx.rectangle(0.0, 0.0, GRAPH_W as f64, GRAPH_H as f64);
    ctx.set_source_surface(&ws.backing_surface.borrow(),
                           0.0 /* offset x */, 0.0 /* offset y */);
    ctx.fill();

    // Calculate FPS, log it once a second.
    ws.fps_count.set(ws.fps_count.get() + 1);
    let now = Instant::now();
    if (now - ws.fps_timer.get()).as_secs() >= 1 {
        debug!("fps: {}", ws.fps_count.get());
        ws.fps_count.set(0);
        ws.fps_timer.set(now);
    }

    Inhibit(false)
}

fn tick(ws: &WindowState) {
    trace!("tick");

    // Ingest new data
    let new_data = ws.data_source.borrow_mut().get_data().unwrap();
    ws.store.borrow_mut().ingest(&*new_data).unwrap();

    let t_latest = ws.store.borrow().last_t();

    // Discard old data if there is any
    let window_base_dt = (GRAPH_W as f32 * BASE_ZOOM_X) as u32;
    if t_latest >= window_base_dt {
        ws.store.borrow_mut().discard(0, t_latest - window_base_dt).unwrap();
    }

    if new_data.len() > 0 {
        // Draw the new data.

        // Calculate the size of the latest patch to render.
        // TODO: Handle when patch_dims.0 > GRAPH_W.
        let patch_dims =
            (((t_latest - ws.last_t_drawn.get()) as f32 / ws.zoom_x.get()).floor() as usize,
             GRAPH_H as usize);
        // If there is more than a pixel's worth of data to render since we last drew,
        // then draw it.
        if patch_dims.0 >= 1 {
            let mut patch_bytes = vec![0u8; patch_dims.0 * patch_dims.1 * BYTES_PER_PIXEL];
            let new_t = ws.last_t_drawn.get() + (patch_dims.0 as f32 * ws.zoom_x.get()) as u32;
            let cols = ws.data_source.borrow().get_colors().unwrap();
            render_patch(&ws.store.borrow(), &cols, &mut patch_bytes,
                         patch_dims.0, patch_dims.1,
                         ws.last_t_drawn.get(), new_t,
                         0, std::u16::MAX).unwrap();

            let patch_offset_x = GRAPH_W - (patch_dims.0 as u32);

            // Copy existing graph to the temp surface, offsetting it to the left.
            {
                let c = cairo::Context::new(&*ws.temp_surface.borrow());
                c.set_source_surface(&*ws.backing_surface.borrow(),
                                     -(patch_dims.0 as f64) /* x offset*/, 0.0 /* y offset */);
                c.rectangle(0.0, // x offset
                            0.0, // y offset
                            patch_offset_x as f64, // width
                            GRAPH_H as f64); // height
                c.fill();
            }
            copy_patch(&ws.temp_surface.borrow(), patch_bytes,
                       patch_dims.0 /* w */, patch_dims.1 /* h */,
                       patch_offset_x as usize /* x */, 0 /* y */);

            // Present new graph by swapping the surfaces.
            ws.backing_surface.swap(&ws.temp_surface);

            ws.last_t_drawn.set(new_t);
        }

        // Invalidate the graph widget so we get a draw request.
        ws.graph_drawing_area.queue_draw();
    }
}

fn render_patch(
    store: &rt_graph::Store, cols: &[rt_graph::Color],
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

            let i = BYTES_PER_PIXEL * (pbw * y + x);
            pb[i+0] = col.0; // R
            pb[i+1] = col.1; // G
            pb[i+2] = col.2; // B
            pb[i+3] = 255;   // A
        }
    }

    Ok(())
}

fn copy_patch(
    backing_surface: &cairo::Surface,
    bytes: Vec<u8>,
    w: usize, h: usize,
    x: usize, y: usize) {
    // Create an ImageSurface from our bytes
    let patch_surface = cairo::ImageSurface::create_for_data(
        bytes,
        cairo::Format::ARgb32,
        w as i32,
        h as i32,
        (w * BYTES_PER_PIXEL) as i32 /* stride */
            ).unwrap();

    // Copy from the ImageSurface to backing_surface
    let c = cairo::Context::new(&backing_surface);
    // Fill target area with background colour.
    c.rectangle(x as f64,
                y as f64,
                w as f64, // width
                h as f64  /* height */);
    c.set_source_rgb(0.0, 0.0, 0.0);
    c.fill_preserve();
    // Fill target area with patch data.
    c.set_source_surface(&patch_surface,
                         x as f64,
                         y as f64);
    c.fill();
}

fn create_backing_surface(win: &gdk::Window, w: u32, h: u32) -> cairo::Surface {
    let surface =
        win.create_similar_image_surface(
            cairo::Format::Rgb24.into(),
            w as i32 /* width */,
            h as i32 /* height */,
            1 /* scale */).unwrap();
    {
        // Clear backing_surface
        let c = cairo::Context::new(&surface);
        c.set_source_rgb(0.4, 0.4, 0.4);
        c.rectangle(0.0, 0.0, w as f64, h as f64);
        c.fill();
    }
    surface
}
