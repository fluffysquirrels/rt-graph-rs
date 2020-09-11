# TODO

## Bugs
* Disable "Follow" button when following.
* Crashes when you zoom in too far.
* Probably shows garbage data when you click to see values after they've been discarded.

## Features
* Mouse over / click to see raw values
* Configure how much old data is stored
* Daniel has 5 graphs, wants them all in sync.
  * Leave it up to controls at a higher level how to navigate, each graph just has show methods.
  * One graph is just the n=1 case
* Configurable larger points (x or + perhaps). Points should be visible on a 4k screen.
* Pause button
* Mouse wheel press to pan
* Mouse wheel to zoom x
* Alt left mouse to zoom box
* Embeddable panel
* `brew install gtk+3` to install dependencies on OS X

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
