# rt-graph

A real-time graphing experiment written in Rust.

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
