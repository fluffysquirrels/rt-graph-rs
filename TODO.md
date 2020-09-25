# TODO

## Bugs

## Features
* Refactor graph click behaviour to GraphWithControls.
* Newtype for Time(u32)
* Document public types
* Example with a non-blocking / fast DataSource
* Example with a blocking DataSource that runs in another thread and ships data over a channel.
* Example with multiple graphs.
* Example with multiple graphs in sync.
* Axes, legend
* Event listeners on Graph: scroll, follow, zoom, _click_.
* When you click on the graph it would be nice to have feedback as to
  where you clicked in the graph.
* Methods on Graph: others? show_point?
* Daniel has 5 graphs, wants them all in sync
  * Leave it up to controls at a higher level how to navigate, each graph just has show methods.
  * Or one graph is just the n=1 case, support GraphSet concept with n `DataSource`s
* Pause button
* Mouse wheel press to pan
* Mouse wheel to zoom x
* Alt left mouse to zoom box
* Export a GLib / GObject interface for consumption by other languages than Rust.
* Scale and offset data (auto-fit to y?)
* Probably use f32 for point data
* Maybe hovering over the graph should show the current point value in a tooltip or sub-window
* Resizing the window should resize the graph.
* Maybe keep the section of the graph that's still valid when scrolling.
* Lower CPU usage when hidden (e.g. minimised). Don't bother drawing.
* Profile
* Web port / rewrite?

## Notes

```
/// Scale value linearly from [0,1] to [min,max]
fn map_to_range(value: f32, min: f32, max: f32) -> f32 {
    value * (max - min) + min
}

/// Scale value linearly from [min, max] to [0,1]
fn normalize(value: f32, min: f32, max: f32) -> f32 {
    let delta = max - min;
    assert!(delta != 0.);

    (value.clamp(min, max) - min) / delta
}
```
