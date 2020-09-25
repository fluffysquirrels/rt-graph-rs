use crate::{Config, Graph, View, ViewMode};
use gdk::prelude::*;
use gtk::prelude::*;
use std::{rc::Rc, cell::RefCell};

pub struct GraphWithControls {
    s: Rc<State>,
}

struct State {
    scrollbar: gtk::Scrollbar,
    btn_zoom_x_out: gtk::Button,
    btn_zoom_x_in: gtk::Button,
    btn_follow: gtk::Button,

    graph: RefCell<Graph>,
}

impl GraphWithControls {
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
            .height_request(50)
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
        // Show everything recursively
        controls_box.show_all();

        g
    }

    fn clone(&self) -> GraphWithControls {
        GraphWithControls {
            s: self.s.clone()
        }
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
    // adj.set_value(0.0);
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
