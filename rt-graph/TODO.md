# TODO

## Bugs
* Making the window wider should keep the scrollbar as the same width as the graph.

## Features
* Event listeners on Graph
* Methods on Graph: scroll, set zoom, get view.
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
* Scale and offset data (auto-fit to y?)
* Probably use f32 for point data

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
