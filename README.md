# rt-graph

A real-time graphing experiment written in Rust.

Many other graphing tools do not efficiently update the display when
new data is added, for example redrawing the whole screen when only a
few pixels of new data are added.

This crate tries to do the minimum incremental work required to update
the graph when new data is added: draw the few pixels of new data, and
scroll the graph with efficient large copies, which can and should be
accelerated by GPU hardware.

As a result of this design rt-graph easily copes with 30k new points
per second, at 60 FPS, using just 3% CPU (tested on a Lenovo T460
laptop with 2.4 GHz Intel Core i5-6300U, running Ubuntu 18.04.5).

## To run

First install GTK 3 dependencies.

On OS X with brew try: `brew install gtk+3`

On Ubuntu try: `sudo apt-get install libgtk-3-dev`

Then try an example with some simulated data:

```
cd ${REPO}/rt-graph
cargo run --package "example-gtk" --release
```

To use your own data implement the DataSource trait and pass an instance of your
struct to the `ConfigBuilder::data_source()` method while building a `Graph`.

## Helpful links

GTK 3 documentation:  <https://developer.gnome.org/gtk3/stable/index.html>

gtk-rs (Rust GTK bindings) documentation: <https://gtk-rs.org/docs-src/>
