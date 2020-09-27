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

    drawing_area: gtk::DrawingArea,

    view_write: RefCell<observable_value::WriteHalf<View>>,
    view_read: RefCell<observable_value::ReadHalf<View>>,

    fps_count: Cell<u16>,
    fps_timer: Cell<Instant>,

    config: Config,
}

#[derive(Clone, Debug)]
pub struct View {
    /// Zoom level, in units of t per x pixel
    pub zoom_x: f64,
    pub last_drawn_t: u32,
    pub last_drawn_x: u32,
    pub min_t: u32,
    pub max_t: u32,
    pub mode: ViewMode,
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
            last_drawn_t: 0,
            last_drawn_x: 0,
            min_t: 0,
            max_t: 0,
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

    #[builder(default = "PointStyle::Point")]
    point_style: PointStyle,
}

#[derive(Clone, Copy, Debug)]
pub enum PointStyle {
    Point,
    Cross,
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

        let drawing_area = gtk::DrawingAreaBuilder::new()
            .height_request(config.graph_height as i32)
            .width_request(config.graph_width as i32)
            .build();
        container.add(&drawing_area);

        // Initialise State

        let backing_surface = create_backing_surface(gdk_window,
                                                     config.graph_width, config.graph_height);
        let temp_surface = create_backing_surface(gdk_window,
                                                  config.graph_width, config.graph_height);
        let store = Store::new(config.data_source.borrow().get_num_values().unwrap() as u8);
        let view = View::default_from_config(&config);
        let (view_read, view_write) =
            observable_value::ObservableValue::new(view.clone()).split();
        let s = Rc::new(State {
            backing_surface: RefCell::new(backing_surface),
            temp_surface: RefCell::new(temp_surface),

            store: RefCell::new(store),

            drawing_area: drawing_area.clone(),

            view_read: RefCell::new(view_read),
            view_write: RefCell::new(view_write),

            fps_count: Cell::new(0),
            fps_timer: Cell::new(Instant::now()),

            config,
        });
        let graph = Graph {
            s: s.clone(),
        };

        // Set signal handlers that require State
        let sc = s.clone();
        drawing_area.connect_draw(move |ctrl, ctx| {
            graph_draw(ctrl, ctx, &*sc)
        });

        // Register our tick timer.
        let sc = s.clone();
        let _tick_id = drawing_area.add_tick_callback(move |_ctrl, _clock| {
            tick(&*sc);
            Continue(true)
        });

        // Show everything recursively
        drawing_area.show_all();

        graph
    }

    pub fn width(&self) -> u32 {
        self.s.config.graph_width
    }

    pub fn height(&self) -> u32 {
        self.s.config.graph_height
    }

    pub fn base_zoom_x(&self) -> f64 {
        self.s.config.base_zoom_x
    }

    pub fn max_zoom_x(&self) -> f64 {
        self.s.config.max_zoom_x
    }

    pub fn view(&self) -> View {
        self.s.view_read.borrow().get()
    }

    pub fn last_t(&self) -> u32 {
        self.s.store.borrow().last_t()
    }

    pub fn first_t(&self) -> u32 {
        self.s.store.borrow().first_t()
    }

    fn _clone(&self) -> Graph {
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

        redraw_graph(&*self.s);
    }

    pub fn set_follow(&self) {
        debug!("set_follow");
        {
            // Scope the mutable borrow of view.
            let new_view = View {
                mode: ViewMode::Following,
                last_drawn_t: self.s.store.borrow().last_t(),
                .. self.s.view_read.borrow().get()
            };
            self.s.view_write.borrow_mut().set(&new_view);
        }
        redraw_graph(&*self.s);
    }

    pub fn scroll(&self, new_val: f64) {
        debug!("scroll new_val={}", new_val);
        {
            // Scope the borrow_mut on view
            let mut view = self.s.view_read.borrow().get();
            view.mode = ViewMode::Scrolled;
            let new_t = (new_val as u32 +
                         ((view.zoom_x * self.s.config.graph_width as f64) as u32))
                .min(self.s.store.borrow().last_t());
            // Snap new_t to a whole pixel.
            let new_t = (((new_t as f64) / view.zoom_x).floor() * view.zoom_x) as u32;
            view.last_drawn_t = new_t;
            view.last_drawn_x = 0;
            self.s.view_write.borrow_mut().set(&view);
            debug!("scroll_change, v={:?} view={:?}", new_val, view);
        }
        // TODO: Maybe keep the section of the graph that's still valid when scrolling.
        redraw_graph(&self.s);
    }

    pub fn view_observable(&mut self) -> RefMut<observable_value::ReadHalf<View>> {
        self.s.view_read.borrow_mut()
    }

    pub fn drawing_area(&self) -> gtk::DrawingArea {
        self.s.drawing_area.clone()
    }

    pub fn drawing_area_pos_to_point(&self, x: f64, _y: f64) -> Option<Point> {
        let view = self.s.view_read.borrow().get();
        let t = (view.last_drawn_t as i64 +
                 ((x - (view.last_drawn_x as f64)) * view.zoom_x) as i64)
            .max(0).min(view.last_drawn_t as i64)
            as u32;
        let pt = self.s.store.borrow().query_point(t).unwrap();

        // If we are getting a point >= 10 pixels away, return None instead.
        // This can happen when old data has been discarded but is still on screen.
        let pt: Option<Point> = if (pt.as_ref().unwrap().t - t) >= (view.zoom_x * 10.0) as u32 {
            None
        } else {
            pt
        };

        pt
    }
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
    let t1: u32 = view.last_drawn_t;
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
                     0 /* v0 */, std::u16::MAX /* v1 */,
                     point_func_select(s.config.point_style));
        view.last_drawn_x = (x + patch_dims.0) as u32;
        view.last_drawn_t = t1;
        s.view_write.borrow_mut().set(&view);
    }
    s.drawing_area.queue_draw();
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

    let mut view = s.view_read.borrow().get();
    view.min_t = s.store.borrow().first_t();
    view.max_t = t_latest;
    s.view_write.borrow_mut().set(&view);

    if new_data.len() > 0 && (view.mode == ViewMode::Following ||
                              (view.mode == ViewMode::Scrolled &&
                               view.last_drawn_x < s.config.graph_width)) {
        // Draw the new data.

        // Calculate the size of the latest patch to render.
        // TODO: Handle when patch_dims.0 >= s.config.graph_width.
        // TODO: Handle scrolled when new data is offscreen (don't draw)
        let patch_dims =
            ((((t_latest - view.last_drawn_t) as f64 / view.zoom_x)
               .floor() as usize)
               .min(s.config.graph_width as usize),
             s.config.graph_height as usize);
        // If there is more than a pixel's worth of data to render since we last drew,
        // then draw it.
        if patch_dims.0 > 0 {
            let new_t = view.last_drawn_t + (patch_dims.0 as f64 * view.zoom_x) as u32;

            let patch_offset_x = match view.mode {
                ViewMode::Following => s.config.graph_width - (patch_dims.0 as u32),
                ViewMode::Scrolled => view.last_drawn_x,
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
                         view.last_drawn_t, new_t,
                         0 /* v0 */, std::u16::MAX /* v1 */,
                         point_func_select(s.config.point_style));

            view.last_drawn_t = new_t;
            view.last_drawn_x = (patch_offset_x + patch_dims.0 as u32).min(s.config.graph_width);
            s.view_write.borrow_mut().set(&view);
        }

        // Invalidate the graph widget so we get a draw request.
        s.drawing_area.queue_draw();
    }
}

fn render_patch(
    surface: &cairo::Surface,
    store: &Store, cols: &[Color],
    pw: usize, ph: usize,
    x: usize, y: usize,
    t0: u32, t1: u32, v0: u16, v1: u16,
    point_func: &dyn Fn(usize, usize, usize, usize, &mut [u8], Color),
) {
    trace!("render_patch: pw={}, ph={} x={} y={}", pw, ph, x, y);
    let mut patch_bytes = vec![0u8; pw * ph * BYTES_PER_PIXEL];
    render_patch_to_bytes(store, cols, &mut patch_bytes,
                          pw, ph,
                          t0, t1,
                          v0, v1,
                          point_func).unwrap();
    copy_patch(surface, patch_bytes,
               pw, ph,
               x, y);
}

fn point_func_select(s: PointStyle) -> &'static dyn Fn(usize, usize, usize, usize, &mut [u8], Color) {
    match s {
        PointStyle::Point => &point_func_point,
        PointStyle::Cross => &point_func_cross,
    }
}

fn point_func_point(x: usize, y: usize, pbw: usize, pbh: usize, pb: &mut [u8], col: Color) {
    if x < pbw && y < pbh {
        let i = BYTES_PER_PIXEL * (pbw * y + x);
        pb[i+0] = col.0; // R
        pb[i+1] = col.1; // G
        pb[i+2] = col.2; // B
        pb[i+3] = 255;   // A
    }
}

fn point_func_cross(x: usize, y: usize, pbw: usize, pbh: usize, pb: &mut [u8], col: Color) {
    let mut pixel = |px: usize, py: usize| {
        if px < pbw && py < pbh {
            let i = BYTES_PER_PIXEL * (pbw * py + px);
            pb[i+0] = col.0; // R
            pb[i+1] = col.1; // G
            pb[i+2] = col.2; // B
            pb[i+3] = 255;   // A
        }
    };

    pixel(x-1, y-1);
    pixel(x+1, y-1);
    pixel(x  , y  );
    pixel(x-1, y+1);
    pixel(x+1, y+1);
}

fn render_patch_to_bytes(
    store: &Store, cols: &[Color],
    pb: &mut [u8], pbw: usize, pbh: usize,
    t0: u32, t1: u32, v0: u16, v1: u16,
    point_func: &dyn Fn(usize, usize, usize, usize, &mut [u8], Color),
) -> Result<()>
{
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
            // Mirror the y-axis
            let y = pbh - y;

            point_func(x, y, pbw, pbh, pb, col);
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
