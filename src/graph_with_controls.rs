use crate::{Config, Graph, View, ViewMode};
use gdk::prelude::*;
use gtk::prelude::*;
use std::{rc::Rc, cell::RefCell};

/// A GTK widget that contains a graph and controls to navigate it.
///
/// If you want a customised graph with your own controls, you might
/// want to try using `Graph`, which is designed for customisation.
pub struct GraphWithControls {
    s: Rc<State>,
}

struct State {
    controls_box: gtk::Box,

    scrollbar: gtk::Scrollbar,
    btn_zoom_x_out: gtk::Button,
    btn_zoom_x_in: gtk::Button,
    btn_follow: gtk::Button,

    graph: RefCell<Graph>,
}

impl GraphWithControls {
    /// Build and show a `GraphWithControls` widget in the target `gtk::Container`.
    pub fn build_ui<C>(config: Config, container: &C, gdk_window: &gdk::Window
    ) -> GraphWithControls
        where C: IsA<gtk::Container> + IsA<gtk::Widget>
    {
        // Create the controls

        let controls_box = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Vertical)
            .spacing(0)
            .build();
        container.add(&controls_box);

        let graph = Graph::build_ui(config, &controls_box, gdk_window);

        let scrollbar = gtk::ScrollbarBuilder::new()
            .orientation(gtk::Orientation::Horizontal)
            .halign(gtk::Align::Start)
            .build();
        scrollbar.set_property_width_request(graph.width() as i32);
        controls_box.add(&scrollbar);

        let buttons_box = gtk::BoxBuilder::new()
            .orientation(gtk::Orientation::Horizontal)
            .height_request(35)
            .build();
        controls_box.add(&buttons_box);

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

        // Set up the state

        let s = Rc::new(State {
            controls_box: controls_box.clone(),

            scrollbar: scrollbar.clone(),
            btn_zoom_x_out: btn_zoom_x_out.clone(),
            btn_zoom_x_in: btn_zoom_x_in.clone(),
            btn_follow: btn_follow.clone(),

            graph: RefCell::new(graph),
        });
        let g = GraphWithControls {
            s: s.clone(),
        };

        update_controls(&g, &g.s.graph.borrow().view());

        // Event handlers that require state.

        let gc = g.clone();
        scrollbar.connect_change_value(move |_ctrl, _scroll_type, v| {
            gc.s.graph.borrow().scroll(v);
            Inhibit(false)
        });

        let gc = g.clone();
        btn_follow.connect_clicked(move |_btn| {
            gc.s.graph.borrow().set_follow()
        });

        let gc = g.clone();
        btn_zoom_x_in.connect_clicked(move |_btn| {
            let new = gc.s.graph.borrow().view().zoom_x / 2.0;
            gc.s.graph.borrow().set_zoom_x(new);
        });

        let gc = g.clone();
        btn_zoom_x_out.connect_clicked(move |_btn| {
            let new = gc.s.graph.borrow().view().zoom_x * 2.0;
            gc.s.graph.borrow().set_zoom_x(new);
        });

        {
            // Scope the borrow on view_observable.
            let gc = g.clone();
            s.graph.borrow_mut().view_observable().connect(move |view| {
                update_controls(&gc, &view);
            });
        }

        let gc = g.clone();
        g.s.graph.borrow().drawing_area().add_events(gdk::EventMask::BUTTON_PRESS_MASK);
        g.s.graph.borrow().drawing_area().connect_button_press_event(move |_ctrl, ev| {
            drawing_area_button_press(&gc, ev)
        });

        // Show everything recursively
        controls_box.show_all();

        g
    }

    fn clone(&self) -> GraphWithControls {
        GraphWithControls {
            s: self.s.clone()
        }
    }

    /// Show the graph and controls.
    pub fn show(&self) {
        self.s.controls_box.show();
        self.s.graph.borrow().show();
    }

    /// Hide the graph and controls.
    pub fn hide(&self) {
        self.s.controls_box.hide();
        self.s.graph.borrow().hide();
    }
}

/// Update the controls (GTK widgets) from the current state.
fn update_controls(g: &GraphWithControls, view: &View) {
    trace!("update_controls view={:?}", view);
    let s = &g.s;
    let adj = s.scrollbar.get_adjustment();
    let window_width_t = (s.graph.borrow().width() as f64) * view.zoom_x;

    adj.set_upper(s.graph.borrow().last_t() as f64);
    adj.set_lower(s.graph.borrow().first_t() as f64);
    adj.set_step_increment(window_width_t / 4.0);
    adj.set_page_increment(window_width_t / 2.0);
    adj.set_page_size(window_width_t);

    match view.mode {
        ViewMode::Following =>
            adj.set_value(s.graph.borrow().last_t() as f64),
        ViewMode::Scrolled => adj.set_value(view.last_drawn_t as f64 -
                                            ((s.graph.borrow().width() as f64) * view.zoom_x)),
    }

    s.btn_zoom_x_in.set_sensitive(view.zoom_x > s.graph.borrow().max_zoom_x());
    s.btn_zoom_x_out.set_sensitive(view.zoom_x < s.graph.borrow().base_zoom_x());
    s.btn_follow.set_sensitive(view.mode == ViewMode::Scrolled);
}

fn drawing_area_button_press(g: &GraphWithControls, ev: &gdk::EventButton) -> Inhibit {
    let pos = ev.get_position();
    let pt = g.s.graph.borrow().drawing_area_pos_to_point(pos.0, pos.1);
    debug!("drawing_area button_press pos={:?} pt={:?}", pos, pt);

    if let Some(pta) = pt {
        let info_bar = gtk::InfoBarBuilder::new()
            .halign(gtk::Align::Start)
            .build();
        g.s.controls_box.add(&info_bar);
        info_bar.set_property_width_request(g.s.graph.borrow().width() as i32);

        info_bar.get_content_area().add(&gtk::Label::new(Some("Time, [Values]:")));

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
        let cbc = g.s.controls_box.clone();
        close_btn.connect_clicked(move |_btn| {
            cbc.remove(&ibc);
        });

        info_bar.show_all();
    }

    Inhibit(false)
}
