use crate::{Color, DataSource, observable_value, Point, Result, Store};
use gdk::prelude::*;
use glib::source::Continue;
use gtk::prelude::*;
use std::{
    cell::{Cell, RefCell, RefMut},
    rc::Rc,
    time::Instant,
};

const BYTES_PER_PIXEL: usize = 4;
const BACKGROUND_COLOR: (f64, f64, f64) = (0.4, 0.4, 0.4);
const DRAWN_AREA_BACKGROUND_COLOR: (f64, f64, f64) = (0.0, 0.0, 0.0);

struct State {
    backing_surface: RefCell<cairo::Surface>,
    temp_surface: RefCell<cairo::Surface>,

    store: RefCell<Store>,

    win_box: gtk::Box,
    graph_drawing_area: gtk::DrawingArea,
    scrollbar: gtk::Scrollbar,
    btn_zoom_x_out: gtk::Button,
    btn_zoom_x_in: gtk::Button,
    btn_follow: gtk::Button,

    view_write: RefCell<observable_value::WriteHalf<View>>,
    view_read: RefCell<observable_value::ReadHalf<View>>,

    fps_count: Cell<u16>,
    fps_timer: Cell<Instant>,

    config: Config,
}

#[derive(Clone, Debug)]
pub struct View {
    /// Zoom level, in units of t per x pixel
    zoom_x: f64,
    last_t: u32,
    last_x: u32,
    mode: ViewMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ViewMode {
    Following,
    Scrolled,
}

impl View {
    fn default_from_config(c: &Config) -> View {
        View {
            zoom_x: c.base_zoom_x,
            last_t: 0,
            last_x: 0,
            mode: ViewMode::Following,
        }
    }
}

#[derive(Builder, Debug)]
#[builder(pattern = "owned")]
pub struct Config {
    /// Maximum zoom out, in units of t per x pixel
    #[builder(default = "1000.0")]
    base_zoom_x: f64,

    /// Maximum zoom in, in units of t per x pixel
    #[builder(default = "1.0")]
    max_zoom_x: f64,

    #[builder(default = "800")]
    graph_width: u32,

    #[builder(default = "200")]
    graph_height: u32,

    #[builder(private, setter(name = "data_source_internal"))]
    data_source: RefCell<Box<dyn DataSource>>,

    /// How many windows width of data to store at maximum zoom out.
    #[builder(default = "100")]
    windows_to_store: u32,
}

impl ConfigBuilder {
    pub fn data_source<T: DataSource + 'static>(self, ds: T) -> Self {
        self.data_source_internal(RefCell::new(Box::new(ds)))
    }
}

pub struct Graph {
    s: Rc<State>,
}

impl Graph {
    /// Build and show a `Graph` widget in the target `gtk::Container`.
    pub fn build_ui<C>(config: Config, container: &C, gdk_window: &gdk::Window) -> Graph
        where C: IsA<gtk::Container> + IsA<gtk::Widget>
    {
        let win_box = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .build();
        container.add(&win_box);

        let drawing_area = gtk::DrawingAreaBuilder::new()
            .height_request(config.graph_height as i32)
            .width_request(config.graph_width as i32)
            .build();
        win_box.add(&drawing_area);

        let scroll = gtk::ScrollbarBuilder::new()
            .orientation(gtk::Orientation::Horizontal)
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

        // Initialise State

        let backing_surface = create_backing_surface(gdk_window,
                                                     config.graph_width, config.graph_height);
        let temp_surface = create_backing_surface(gdk_window,
                                                  config.graph_width, config.graph_height);
        let store = Store::new(config.data_source.borrow().get_num_values().unwrap() as u8);
        let view = View::default_from_config(&config);
        let (view_read, view_write) =
            observable_value::ObservableValue::new(view).split();
        let s = Rc::new(State {
            backing_surface: RefCell::new(backing_surface),
            temp_surface: RefCell::new(temp_surface),

            store: RefCell::new(store),

            win_box: win_box.clone(),
            graph_drawing_area: drawing_area.clone(),
            scrollbar: scroll.clone(),
            btn_zoom_x_out: btn_zoom_x_out.clone(),
            btn_zoom_x_in: btn_zoom_x_in.clone(),
            btn_follow: btn_follow.clone(),

            view_read: RefCell::new(view_read),
            view_write: RefCell::new(view_write),

            fps_count: Cell::new(0),
            fps_timer: Cell::new(Instant::now()),

            config,
        });
        let graph = Graph {
            s: s.clone(),
        };

        update_controls(&*s);

        // Set signal handlers that require State
        let sc = s.clone();
        drawing_area.connect_draw(move |ctrl, ctx| {
            graph_draw(ctrl, ctx, &*sc)
        });

        let sc = s.clone();
        drawing_area.add_events(gdk::EventMask::BUTTON_PRESS_MASK);
        drawing_area.connect_button_press_event(move |_ctrl, ev| {
            graph_click(&*sc, ev)
        });
        // drawing_area.add_events(gdk::EventMask::POINTER_MOTION_MASK);
        // drawing_area.connect_motion_notify_event(move |ctrl, ev| {
        //     debug!("drawing_area_mouse_move ev.pos={:?}", ev.get_position());
        //     Inhibit(false)
        // });

        // Register our tick timer.
        let sc = s.clone();
        let _tick_id = win_box.add_tick_callback(move |_ctrl, _clock| {
            tick(&*sc);
            Continue(true)
        });

        let gc = graph.clone();
        scroll.connect_change_value(move |_ctrl, _scroll_type, v| {
            gc.scroll(v);
            Inhibit(false)
        });

        let gc = graph.clone();
        btn_follow.connect_clicked(move |_btn| {
            gc.set_follow()
        });

        let gc = graph.clone();
        btn_zoom_x_in.connect_clicked(move |_btn| {
            let new = gc.s.view_read.borrow().get().zoom_x / 2.0;
            gc.set_zoom_x(new);
        });

        let gc = graph.clone();
        btn_zoom_x_out.connect_clicked(move |_btn| {
            let new = gc.s.view_read.borrow().get().zoom_x * 2.0;
            gc.set_zoom_x(new);
        });

        // Show everything recursively
        win_box.show_all();

        graph
    }

    fn clone(&self) -> Graph {
        Graph {
            s: self.s.clone()
        }
    }

    pub fn set_zoom_x(&self, new_zoom_x: f64) {
        debug!("set_zoom_x new_zoom_x={}", new_zoom_x);
        let new_zoom_x = new_zoom_x.min(self.s.config.base_zoom_x)
            .max(self.s.config.max_zoom_x);
        {
            // Scope the mutable borrow of view.
            let new_view = View {
                zoom_x: new_zoom_x,
                .. self.s.view_read.borrow().get()
            };
            self.s.view_write.borrow_mut().set(&new_view);
        }
        update_controls(&*self.s);

        redraw_graph(&*self.s);
    }

    pub fn set_follow(&self) {
        debug!("set_follow");
        {
            // Scope the mutable borrow of view.
            let new_view = View {
                mode: ViewMode::Following,
                last_t: self.s.store.borrow().last_t(),
                .. self.s.view_read.borrow().get()
            };
            self.s.view_write.borrow_mut().set(&new_view);
            self.s.scrollbar.set_value(new_view.last_t as f64);
        }
        update_controls(&*self.s);
        redraw_graph(&*self.s);
    }

    pub fn scroll(&self, new_val: f64) {
        {
            // Scope the borrow_mut on view
            let mut view = self.s.view_read.borrow().get();
            view.mode = if new_val >= self.s.scrollbar.get_adjustment().get_upper() - 1.0 {
                ViewMode::Following
            } else {
                ViewMode::Scrolled
            };

            let new_t = (new_val as u32 +
                         ((view.zoom_x * self.s.config.graph_width as f64) as u32))
                .min(self.s.store.borrow().last_t());
            // Snap new_t to a whole pixel.
            let new_t = (((new_t as f64) / view.zoom_x).floor() * view.zoom_x) as u32;
            view.last_t = new_t;
            view.last_x = 0;
            self.s.view_write.borrow_mut().set(&view);
            debug!("scroll_change, v={:?} view={:?}", new_val, view);
        }
        update_controls(&self.s);
        // TODO: Maybe keep the section of the graph that's still valid when scrolling.
        redraw_graph(&self.s);
    }

    pub fn view_observable(&mut self) -> RefMut<observable_value::ReadHalf<View>> {
        self.s.view_read.borrow_mut()
    }
}

/// Update the controls (GTK widgets) from the current state.
fn update_controls(s: &State) {
    let view = s.view_read.borrow().get();
    let adj = s.scrollbar.get_adjustment();
    let window_width_t = (s.config.graph_width as f64) * view.zoom_x;

    adj.set_upper(s.store.borrow().last_t() as f64);
    adj.set_lower(s.store.borrow().first_t() as f64);
    adj.set_step_increment(window_width_t / 4.0);
    adj.set_page_increment(window_width_t / 2.0);
    adj.set_page_size(window_width_t);
    match view.mode {
        ViewMode::Following =>
            adj.set_value(s.store.borrow().last_t() as f64),
        ViewMode::Scrolled => adj.set_value(view.last_t as f64 -
                                            ((s.config.graph_width as f64) * view.zoom_x)),
    }

    s.btn_zoom_x_in.set_sensitive(view.zoom_x > s.config.max_zoom_x);
    s.btn_zoom_x_out.set_sensitive(view.zoom_x < s.config.base_zoom_x);
    s.btn_follow.set_sensitive(view.mode == ViewMode::Scrolled);
}

fn graph_click(s: &State, ev: &gdk::EventButton) -> Inhibit {
    let pos = ev.get_position();
    let view = s.view_read.borrow().get();
    let t = (view.last_t as i64 +
             ((pos.0 - (view.last_x as f64)) * view.zoom_x) as i64)
             .max(0).min(view.last_t as i64)
        as u32;
    let pt = s.store.borrow().query_point(t).unwrap();

    // If we are getting a point >= 10 pixels away, return None instead.
    // This can happen when old data has been discarded but is still on screen.
    let pt: Option<Point> = if (pt.as_ref().unwrap().t - t) >= (view.zoom_x * 10.0) as u32 {
        None
    } else {
        pt
    };
    debug!("graph_button_press pos={:?} last_t={} last_x={}", pos, view.last_t, view.last_x);
    debug!("graph_button_press t={} pt={:?}", t, pt);

    if let Some(pta) = pt {
        let info_bar = gtk::InfoBarBuilder::new()
            .build();
        s.win_box.add(&info_bar);
        info_bar.get_content_area().add(&gtk::Label::new(Some("t, vs:")));

        let entry = gtk::EntryBuilder::new()
            .text(&*format!("{}, {:?}", pta.t, pta.vals()))
            .editable(false)
            .hexpand(true)
            .build();
        info_bar.get_content_area().add(&entry);

        let close_btn = gtk::ButtonBuilder::new()
            .label("Close")
            .build();
        info_bar.get_action_area().unwrap().add(&close_btn);

        let ibc = info_bar.clone();
        let wbc = s.win_box.clone();
        close_btn.connect_clicked(move |_btn| {
            wbc.remove(&ibc);
        });

        info_bar.show_all();
    }

    Inhibit(false)
}

/// Handle the graph's draw signal.
fn graph_draw(_ctrl: &gtk::DrawingArea, ctx: &cairo::Context, s: &State) -> Inhibit {
    trace!("graph_draw");

    // Copy from the backing_surface, which was updated elsewhere
    ctx.rectangle(0.0, 0.0, s.config.graph_width as f64, s.config.graph_height as f64);
    ctx.set_source_surface(&s.backing_surface.borrow(),
                           0.0 /* offset x */, 0.0 /* offset y */);
    ctx.fill();

    // Calculate FPS, log it once a second.
    s.fps_count.set(s.fps_count.get() + 1);
    let now = Instant::now();
    if (now - s.fps_timer.get()).as_secs() >= 1 {
        debug!("fps: {}", s.fps_count.get());
        s.fps_count.set(0);
        s.fps_timer.set(now);
    }

    Inhibit(false)
}

/// Redraw the whole graph to the backing store
fn redraw_graph(s: &State) {
    trace!("redraw_graph");
    let backing_surface = s.backing_surface.borrow();
    {
        // Clear backing_surface
        let c = cairo::Context::new(&*backing_surface);
        c.set_source_rgb(BACKGROUND_COLOR.0,
                         BACKGROUND_COLOR.1,
                         BACKGROUND_COLOR.2);
        c.rectangle(0.0, 0.0, s.config.graph_width as f64, s.config.graph_height as f64);
        c.fill();
    }

    let mut view = s.view_read.borrow().get();
    let cols = s.config.data_source.borrow().get_colors().unwrap();
    let t1: u32 = view.last_t;
    let t0: u32 = (t1 as i64 - (s.config.graph_width as f64 * view.zoom_x) as i64).max(0) as u32;
    let patch_dims = ((((t1-t0) as f64 / view.zoom_x).floor() as u32)
                          .min(s.config.graph_width) as usize,
                      s.config.graph_height as usize);
    if patch_dims.0 > 0 {
        let x = match view.mode {
            ViewMode::Following => (s.config.graph_width as usize) - patch_dims.0,
            ViewMode::Scrolled => 0,
        };
        render_patch(&*backing_surface,
                     &s.store.borrow(),
                     &cols,
                     patch_dims.0 /* w */, patch_dims.1 /* h */,
                     x /* x */, 0 /* y */,
                     t0, t1,
                     0 /* v0 */, std::u16::MAX /* v1 */);
        view.last_x = (x + patch_dims.0) as u32;
        view.last_t = t1;
        s.view_write.borrow_mut().set(&view);
    }
    s.graph_drawing_area.queue_draw();
}

fn tick(s: &State) {
    trace!("tick");

    // Ingest new data
    let new_data = s.config.data_source.borrow_mut().get_data().unwrap();
    s.store.borrow_mut().ingest(&*new_data).unwrap();

    let t_latest = s.store.borrow().last_t();

    // Discard old data if there is any
    let window_base_dt = (s.config.graph_width as f64 * s.config.base_zoom_x) as u32;
    let keep_window = s.config.windows_to_store * window_base_dt;
    let discard_start = if t_latest >= keep_window { t_latest - keep_window } else { 0 };
    if discard_start > 0 {
        s.store.borrow_mut().discard(0, discard_start).unwrap();
    }

    update_controls(s);

    let mut view = s.view_read.borrow().get();

    if new_data.len() > 0 && (view.mode == ViewMode::Following ||
                              (view.mode == ViewMode::Scrolled && view.last_x < s.config.graph_width)) {
        // Draw the new data.

        // Calculate the size of the latest patch to render.
        // TODO: Handle when patch_dims.0 >= s.config.graph_width.
        // TODO: Handle scrolled when new data is offscreen (don't draw)
        let patch_dims =
            (((t_latest - view.last_t) as f64 / view.zoom_x).floor() as usize,
             s.config.graph_height as usize);
        // If there is more than a pixel's worth of data to render since we last drew,
        // then draw it.
        if patch_dims.0 > 0 {
            let new_t = view.last_t + (patch_dims.0 as f64 * view.zoom_x) as u32;

            let patch_offset_x = match view.mode {
                ViewMode::Following => s.config.graph_width - (patch_dims.0 as u32),
                ViewMode::Scrolled => view.last_x,
            };

            if view.mode == ViewMode::Following {
                // Copy existing graph to the temp surface, offsetting it to the left.
                let c = cairo::Context::new(&*s.temp_surface.borrow());
                c.set_source_surface(&*s.backing_surface.borrow(),
                                     -(patch_dims.0 as f64) /* x offset*/, 0.0 /* y offset */);
                c.rectangle(0.0, // x offset
                            0.0, // y offset
                            patch_offset_x as f64, // width
                            s.config.graph_height as f64); // height
                c.fill();

                // Present new graph by swapping the surfaces.
                s.backing_surface.swap(&s.temp_surface);
            }

            let cols = s.config.data_source.borrow().get_colors().unwrap();
            render_patch(&s.backing_surface.borrow(),
                         &s.store.borrow(),
                         &cols,
                         patch_dims.0 /* w */, patch_dims.1 /* h */,
                         patch_offset_x as usize, 0 /* y */,
                         view.last_t, new_t,
                         0 /* v0 */, std::u16::MAX /* v1 */);

            view.last_t = new_t;
            view.last_x = (patch_offset_x + patch_dims.0 as u32).min(s.config.graph_width);
            s.view_write.borrow_mut().set(&view);
        }

        // Invalidate the graph widget so we get a draw request.
        s.graph_drawing_area.queue_draw();
    }
}

fn render_patch(
    surface: &cairo::Surface,
    store: &Store, cols: &[Color],
    pw: usize, ph: usize,
    x: usize, y: usize,
    t0: u32, t1: u32, v0: u16, v1: u16
) {
    let mut patch_bytes = vec![0u8; pw * ph * BYTES_PER_PIXEL];
    render_patch_to_bytes(store, cols, &mut patch_bytes,
                          pw, ph,
                          t0, t1,
                          v0, v1).unwrap();
    copy_patch(surface, patch_bytes,
               pw, ph,
               x, y);
}

fn render_patch_to_bytes(
    store: &Store, cols: &[Color],
    pb: &mut [u8], pbw: usize, pbh: usize,
    t0: u32, t1: u32, v0: u16, v1: u16
) -> Result<()> {

    trace!("render_patch_to_bytes: pbw={}", pbw);
    assert!(pbw >= 1);
    let points = store.query_range(t0, t1)?;
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
    x: usize, y: usize
) {

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
    c.set_source_rgb(DRAWN_AREA_BACKGROUND_COLOR.0,
                     DRAWN_AREA_BACKGROUND_COLOR.1,
                     DRAWN_AREA_BACKGROUND_COLOR.2);
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
