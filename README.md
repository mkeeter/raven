# Uxn
The Uxn CPU is implemented in the `raven-uxn` crate.

This implementation is focused on speed, safety, and correctness.

It is written in Rust as a `#[no_std]` crate, so it can used as part of a
bare-metal system.  A `#![forbid(unsafe_code)]` annotation ensures that the
crate only uses safe Rust, and the crate has no dependencies.

## Performance
Running the
[`mandelbrot.tal` demo](https://git.sr.ht/~rabbits/uxn/tree/main/item/projects/examples/demos/mandelbrot.tal)
at max scale (`#0020`), `raven-gui` is about 20% faster than the `uxnemu`
reference implementation: it calculates the fractal in 1.60 seconds, versus 2.03
seconds for `uxnemu`.

## Design


# Varvara
## Devices
### Audio
#### Implementation notes
The reference implementation is very different from the specification!

### Datetime
#### Limitations
The `IS_DST` bit always returns 0
(see [`chrono#1562`](https://github.com/chronotope/chrono/issues/1562))
