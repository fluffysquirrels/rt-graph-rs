#[macro_use]
extern crate log;

use gdk::prelude::*;
use gio::prelude::*;
use glib::source::Continue;
use gtk::prelude::*;
use rt_graph::{DataSource, Point};
use std::{
    cell::{Cell, RefCell},
    env::args,
    rc::Rc,
    time::Instant,
};

const GRAPH_W: u32 = 800;
const GRAPH_H: u32 = 200;
const BASE_ZOOM_X: f64 = 1000.0;
const BYTES_PER_PIXEL: usize = 4;
const BACKGROUND_COLOR: (f64, f64, f64) = (0.4, 0.4, 0.4);

struct WindowState {
    backing_surface: RefCell<cairo::Surface>,
    temp_surface: RefCell<cairo::Surface>,

    data_source: RefCell<Box<dyn rt_graph::DataSource>>,
    store: RefCell<rt_graph::Store>,

    graph_drawing_area: gtk::DrawingArea,
    scrollbar: gtk::Scrollbar,
    btn_zoom_x_out: gtk::Button,

    view: RefCell<View>,

    fps_count: Cell<u16>,
    fps_timer: Cell<Instant>,

    windows_to_store: u32,
}

#[derive(Debug)]
struct View {
    /// t per pixel
    zoom_x: f64,
    last_t: u32,
    last_x: u32,
    mode: ViewMode,
}

#[derive(Debug, Eq, PartialEq)]
enum ViewMode {
    Following,
    Scrolled,
}

impl View {
    fn default() -> View {
        View {
            zoom_x: BASE_ZOOM_X,
            last_t: 0,
            last_x: 0,
            mode: ViewMode::Following,
        }
    }
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
    let view = View::default();

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
        .spacing(0)
        .build();
    window.add(&win_box);

    let graph = gtk::DrawingAreaBuilder::new()
        .height_request(GRAPH_H as i32)
        .width_request(GRAPH_W as i32)
        .build();
    win_box.add(&graph);

    let scroll = gtk::ScrollbarBuilder::new()
        .orientation(gtk::Orientation::Horizontal)
        .adjustment(&gtk::Adjustment::new(
            0.0,                                  // value
            0.0,                                  // lower
            0.0,                                  // upper
            (GRAPH_W as f64) * view.zoom_x / 4.0, // step_increment
            (GRAPH_W as f64) * view.zoom_x / 2.0, // page_increment
            (GRAPH_W as f64) * view.zoom_x))      // page_size
        .build();
    win_box.add(&scroll);

    let buttons_box = gtk::BoxBuilder::new()
        .orientation(gtk::Orientation::Horizontal)
        .height_request(50)
        .build();
    win_box.add(&buttons_box);

    let btn_follow = gtk::ButtonBuilder::new()
        .label("Follow")
        .build();
    buttons_box.add(&btn_follow);

    let btn_zoom_x_in = gtk::ButtonBuilder::new()
        .label("Zoom X in")
        .build();
    buttons_box.add(&btn_zoom_x_in);

    let btn_zoom_x_out = gtk::ButtonBuilder::new()
        .label("Zoom X out")
        .sensitive(false)
        .build();
    buttons_box.add(&btn_zoom_x_out);

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
        scrollbar: scroll.clone(),
        btn_zoom_x_out: btn_zoom_x_out.clone(),

        view: RefCell::new(View::default()),

        fps_count: Cell::new(0),
        fps_timer: Cell::new(Instant::now()),

        windows_to_store: 100,
    });

    // Set signal handlers that require WindowState
    let wsc = ws.clone();
    graph.connect_draw(move |ctrl, ctx| {
        graph_draw(ctrl, ctx, &*wsc)
    });

    let wsc = ws.clone();
    graph.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
    graph.connect_button_press_event(move |_ctrl, ev| {
        graph_click(&*wsc, ev)
    });
    // graph.add_events(gdk::EventMask::POINTER_MOTION_MASK);
    // graph.connect_motion_notify_event(move |ctrl, ev| {
    //     debug!("graph_mouse_move ev.pos={:?}", ev.get_position());
    //     Inhibit(false)
    // });

    let wsc = ws.clone();
    let _tick_id = glib::source::timeout_add_local(16 /* ms */, move || {
        tick(&*wsc);
        Continue(true)
    });

    let wsc = ws.clone();
    scroll.connect_change_value(move |ctrl, _scroll_type, v| {
        scroll_change(ctrl, v, &*wsc)
    });

    let wsc = ws.clone();
    btn_follow.connect_clicked(move |_btn| {
        {
            // Scope the mutable borrow of view.
            let mut view = wsc.view.borrow_mut();
            view.mode = ViewMode::Following;
            view.last_t = wsc.store.borrow().last_t();
            scroll.set_value(view.last_t as f64);
        }
        redraw_graph(&*wsc);
    });

    let wsc = ws.clone();
    btn_zoom_x_in.connect_clicked(move |_btn| {
        let new = wsc.view.borrow().zoom_x / 2.0;
        set_zoom_x(&*wsc, new);
    });

    let wsc = ws.clone();
    btn_zoom_x_out.connect_clicked(move |_btn| {
        let new = wsc.view.borrow().zoom_x * 2.0;
        set_zoom_x(&*wsc, new);
    });

    // Show everything recursively
    window.show_all();
}

fn graph_click(ws: &WindowState, ev: &gdk::EventButton) -> Inhibit {
    let pos = ev.get_position();
    let view = ws.view.borrow();
    let t = (view.last_t as i64 +
             ((pos.0 - (view.last_x as f64)) * view.zoom_x) as i64)
             .max(0).min(view.last_t as i64)
        as u32;
    let pt = ws.store.borrow().query_point(t).unwrap();
    // If we are getting a point >= 10 pixels away, return None instead.
    let pt: Option<Point> = if (pt.as_ref().unwrap().t - t) >= (view.zoom_x * 10.0) as u32 {
        None
    } else {
        pt
    };
    debug!("graph_button_press pos={:?} last_t={} last_x={}", pos, view.last_t, view.last_x);
    debug!("graph_button_press t={} pt={:?}", t, pt);

    Inhibit(false)
}

fn set_zoom_x(ws: &WindowState, new_zoom_x: f64) {
    let new_zoom_x = new_zoom_x.min(BASE_ZOOM_X);
    {
        // Scope the mutable borrow of view.
        let mut view = ws.view.borrow_mut();
        view.zoom_x = new_zoom_x;
    }
    let adj = ws.scrollbar.get_adjustment();
    adj.set_step_increment((GRAPH_W as f64) * new_zoom_x / 4.0);
    adj.set_page_increment((GRAPH_W as f64) * new_zoom_x / 2.0);
    adj.set_page_size((GRAPH_W as f64) * new_zoom_x);

    ws.btn_zoom_x_out.set_sensitive(new_zoom_x < BASE_ZOOM_X);

    redraw_graph(&*ws);
}

fn scroll_change(ctrl: &gtk::Scrollbar, new_val: f64, ws: &WindowState) -> Inhibit {
    {
        // Scope the borrow_mut on view
        let mut view = ws.view.borrow_mut();
        view.mode = if new_val >= ctrl.get_adjustment().get_upper() - 1.0 {
            ViewMode::Following
        } else {
            ViewMode::Scrolled
        };
        view.last_t = (new_val as u32 + ((view.zoom_x * GRAPH_W as f64) as u32))
        .min(ws.store.borrow().last_t());
        view.last_x = 0;

        debug!("scroll_change, v={:?} view={:?}", new_val, view);
    }
    // TODO: Maybe keep the section of the graph that's still valid when scrolling.
    redraw_graph(&ws);
    Inhibit(false)
}

/// Handle the graph's draw signal.
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

/// Redraw the whole graph to the backing store
fn redraw_graph(ws: &WindowState) {
    trace!("redraw_graph");
    let backing_surface = ws.backing_surface.borrow();
    {
        // Clear backing_surface
        let c = cairo::Context::new(&*backing_surface);
        c.set_source_rgb(BACKGROUND_COLOR.0,
                         BACKGROUND_COLOR.1,
                         BACKGROUND_COLOR.2);
        c.rectangle(0.0, 0.0, GRAPH_W as f64, GRAPH_H as f64);
        c.fill();
    }

    let mut view = ws.view.borrow_mut();
    let cols = ws.data_source.borrow().get_colors().unwrap();
    let t1: u32 = view.last_t;
    let t0: u32 = (t1 as i64 - (GRAPH_W as f64 * view.zoom_x) as i64).max(0) as u32;
    let patch_dims = ((((t1-t0) as f64 / (view.zoom_x as f64)) as u32).min(GRAPH_W) as usize,
                      GRAPH_H as usize);
    if patch_dims.0 > 0 {
        let mut patch_bytes = vec![0u8; patch_dims.0 * patch_dims.1 * BYTES_PER_PIXEL];
        render_patch(&*ws.store.borrow(), &cols, &mut patch_bytes,
                     patch_dims.0, patch_dims.1,
                     t0, t1,
                     0, std::u16::MAX). unwrap();
        copy_patch(&*backing_surface, patch_bytes,
                   patch_dims.0, patch_dims.1,
                   0 /* x */, 0 /* y */);
        view.last_x = patch_dims.0 as u32;
    }
    ws.graph_drawing_area.queue_draw();
}

fn tick(ws: &WindowState) {
    trace!("tick");

    // Ingest new data
    let new_data = ws.data_source.borrow_mut().get_data().unwrap();
    ws.store.borrow_mut().ingest(&*new_data).unwrap();

    let t_latest = ws.store.borrow().last_t();

    // Discard old data if there is any
    let window_base_dt = (GRAPH_W as f64 * BASE_ZOOM_X) as u32;
    let keep_window = ws.windows_to_store * window_base_dt;
    let discard_start = if t_latest >= keep_window { t_latest - keep_window } else { 0 };
    if discard_start > 0 {
        ws.store.borrow_mut().discard(0, discard_start).unwrap();
    }

    let mut view = ws.view.borrow_mut();

    // Update scroll bar.
    ws.scrollbar.get_adjustment().set_upper(t_latest as f64);
    ws.scrollbar.get_adjustment().set_lower(discard_start as f64);
    if view.mode == ViewMode::Following {
        ws.scrollbar.set_value(t_latest as f64);
    }

    if new_data.len() > 0 && (view.mode == ViewMode::Following ||
                              (view.mode == ViewMode::Scrolled && view.last_x < GRAPH_W)) {
        // Draw the new data.

        // Calculate the size of the latest patch to render.
        // TODO: Handle when patch_dims.0 >= GRAPH_W.
        // TODO: Handle scrolled when new data is offscreen (don't draw)
        let patch_dims =
            (((t_latest - view.last_t) as f64 / view.zoom_x).floor() as usize,
             GRAPH_H as usize);
        // If there is more than a pixel's worth of data to render since we last drew,
        // then draw it.
        if patch_dims.0 > 0 {
            let mut patch_bytes = vec![0u8; patch_dims.0 * patch_dims.1 * BYTES_PER_PIXEL];
            let new_t = view.last_t + (patch_dims.0 as f64 * view.zoom_x) as u32;
            let cols = ws.data_source.borrow().get_colors().unwrap();
            render_patch(&ws.store.borrow(), &cols, &mut patch_bytes,
                         patch_dims.0, patch_dims.1,
                         view.last_t, new_t,
                         0, std::u16::MAX).unwrap();

            let patch_offset_x = match view.mode {
                ViewMode::Following => GRAPH_W - (patch_dims.0 as u32),
                ViewMode::Scrolled => view.last_x,
            };

            if view.mode == ViewMode::Following {
                // Copy existing graph to the temp surface, offsetting it to the left.
                let c = cairo::Context::new(&*ws.temp_surface.borrow());
                c.set_source_surface(&*ws.backing_surface.borrow(),
                                     -(patch_dims.0 as f64) /* x offset*/, 0.0 /* y offset */);
                c.rectangle(0.0, // x offset
                            0.0, // y offset
                            patch_offset_x as f64, // width
                            GRAPH_H as f64); // height
                c.fill();

                // Present new graph by swapping the surfaces.
                ws.backing_surface.swap(&ws.temp_surface);
            }
            copy_patch(&ws.backing_surface.borrow(), patch_bytes,
                       patch_dims.0 /* w */, patch_dims.1 /* h */,
                       patch_offset_x as usize /* x */, 0 /* y */);

            view.last_t = new_t;
            view.last_x = (patch_offset_x + patch_dims.0 as u32).min(GRAPH_W);
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
    let points = store.query_range(t0, t1).unwrap();
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

    trace!("copy_patch w={} x={}", w, x);

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
        c.set_source_rgb(BACKGROUND_COLOR.0,
                         BACKGROUND_COLOR.1,
                         BACKGROUND_COLOR.2);
        c.rectangle(0.0, 0.0, w as f64, h as f64);
        c.fill();
    }
    surface
}
