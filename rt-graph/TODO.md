# TODO

## Bugs
* Scrolling overdraws weird stuff sometimes
* Zoom x out should show disabled when we're at furthest zoom out level
* Flickering when scrolling
* Continue to draw when scrolled and new data is on screen
* Sometimes crashes when zoomed in far and scrolling "thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: InvalidSize', examples/gtk/src/main.rs:420:25
" on copy\_patch's call to cairo::ImageSurface::create\_for\_data

## Features
* Pause button
* Mouse wheel press to pan
* Mouse wheel to zoom x
* Alt left mouse to zoom box
* Mouse over / click to see raw values
* Embeddable panel
* Daniel has 5 graphs, wants them all in sync.
  * Leave it up to controls at a higher level how to navigate, each graph just has show methods.
  * One graph is just the n=1 case
* `brew install gtk+3` to install dependencies on OS X
