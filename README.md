# A GNU Rocket client library

A [GNU Rocket Editor][rocket] client implementation in Rust. This connects to a
running Rocket editor from a demo tool over `localhost:1338`.

Note that the [rocket_sync] lib is also necessary, which gives you the value of
a sync track at a given time.

[rocket]: https://github.com/emoon/rocket
[rocket_sync]: https://github.com/make-a-demo-tool-in-rust/rocket_sync

## Getting Started

Download or compile the [Rocket Editor][rocket], instructions at the repo's README.

Copy the binary to somewhere in the `$PATH`.

Start Rocket, this starts a listener on `localhost:1338`.

```
rocket_editor &
```

Start the tool which connects to it as a client.

See `examples/basic_example.rs`, this creates a list of tracks, listens to
changes and prints their value at the current time.

```
cargo run --example basic_example
```

![Rocket client demo](images/rocket-client-demo.gif)
