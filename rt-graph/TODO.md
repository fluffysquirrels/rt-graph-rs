# TODO

## Bugs

## Features
* Rename example-gtk.
* Event listeners on Graph: scroll, follow, zoom, click.
* Methods on Graph: others? show_point?
* Daniel has 5 graphs, wants them all in sync.
  * Leave it up to controls at a higher level how to navigate, each graph just has show methods.
  * Or one graph is just the n=1 case, support GraphSet concept with n `DataSource`s
* Pause button
* Mouse wheel press to pan
* Mouse wheel to zoom x
* Alt left mouse to zoom box
* Embeddable panel
* Scale and offset data (auto-fit to y?)
* Probably use f32 for point data
* Maybe hovering over the graph should show the current point value in a tooltip or sub-window
* Resizing the window should resize the graph.
* Maybe keep the section of the graph that's still valid when scrolling.
* Lower CPU usage when hidden (e.g. minimised). Don't bother drawing.

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
