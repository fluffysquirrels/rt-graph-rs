# TODO

## Bugs
* Follow before screen is filled leaves a background colour-filled
  hole. (call redraw, redraw handles ViewMode::Following).
* Making the window wider should keep the scrollbar as the same width as the graph.
* build_ui: showing our parent is rude / unexpected. Also I doubt it'll work for some
  detached container.

## Features
* Event listeners on Graph
* Lower CPU usage when hidden (e.g. minimised). Don't bother drawing.
* Daniel has 5 graphs, wants them all in sync.
  * Leave it up to controls at a higher level how to navigate, each graph just has show methods.
  * One graph is just the n=1 case
* Configurable larger points (x or + perhaps). Points should be visible on a 4k screen.
* Pause button
* Mouse wheel press to pan
* Mouse wheel to zoom x
* Alt left mouse to zoom box
* Embeddable panel
* Scale and offset data (auto-fill y?)
* Probably use f32 for point data
* Use the frame clock or add\_tick\_callback for timing instead of
  just rendering every 16ms with glib::source::timeout\_add\_local.
  See:
  https://developer.gnome.org/gtk3/stable/GtkWidget.html#gtk-widget-get-frame-clock ,
  https://developer.gnome.org/gtk3/stable/GtkWidget.html#gtk-widget-add-tick-callback
* Add links to the GTK docs
  https://developer.gnome.org/gtk3/stable/index.html
  https://gtk-rs.org/docs-src/

## Notes

```
fn map_to_range(value: f32, min: f32, max: f32) -> f32 {
    value * (max - min) + min
}

fn normalize(value: f32, min: f32, max: f32) -> f32 {
    let delta = max - min;
    assert!(delta != 0.);

    (value.clamp(min, max) - min) / delta
}
```
