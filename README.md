**Raven** is an independent re-implementation of the
[Uxn CPU](https://wiki.xxiivv.com/site/uxn.html)
and
[Varvara Ordinator](https://wiki.xxiivv.com/site/varvara.html).

For details, see [the project writeup](https://mattkeeter.com/projects/raven).

--------------------------------------------------------------------------------

The `raven-uxn` crate includes three implementations of the Uxn CPU:

- The safe interpreter is a `#[no_std]` crate written in 100% safe Rust, with a
  single dependency (`zerocopy`).  It is 10-20% faster than
  the [reference implementation](https://git.sr.ht/~rabbits/uxn/tree/main/item/src)
  for CPU-heavy workloads (e.g.
  [`fib.tal`](https://git.sr.ht/~rabbits/uxn/tree/main/item/projects/examples/exercises/fib.tal),
  and
  [`mandelbrot.tal`](https://git.sr.ht/~rabbits/uxn/tree/main/item/projects/examples/demos/mandelbrot.tal)
- The unsafe ("native") interpreters are written in `aarch64` and `x86-64`
  assembly (with Rust shims on either side), and run *significantly* faster
  than the reference implementation (40-50% faster for AArch64, 2× faster for
  x86)

The native interpreter can be checked against the safe interpreter with fuzz
testing:

```console
cargo install cargo-fuzz # this only needs to be run once
cargo +nightly fuzz run --release fuzz-native
# alternatively, run `just fuzz`
```

Performance can be tested with `cargo bench`, which runs
[Criterion.rs](https://criterion-rs.github.io/)-based benchmarks
for recursive Fibonacci computation and Mandelbrot fractal rendering.

--------------------------------------------------------------------------------

The Varvara implementation (`raven-varvara`) includes all peripherals, and has
been tested on many of the
[flagship applications](https://wiki.xxiivv.com/site/roms.html)
(Left, Orca, Noodle, Potato).

--------------------------------------------------------------------------------

The repository includes two applications built on these libraries:

- `raven-cli` is a command-line application to run console-based ROMs
- `raven-gui` is a full-fledged GUI, which runs both as a native application and
  [on the web](https://mattkeeter.com/projects/raven/demo)

Building the web application requires
[`wasm-bindgen`](https://rustwasm.github.io/wasm-bindgen/reference/cli.html)
and [`wasm-opt`](https://github.com/WebAssembly/binaryen) to be installed.
Using the [Just](https://just.systems/) command runner, the web application can
be built with `just dist` (deploying files to `raven-gui/dist`) and served
locally with `just serve`.

--------------------------------------------------------------------------------

© 2024-2026 Matthew Keeter  
Released under the [Mozilla Public License 2.0](https://github.com/mkeeter/fidget/blob/main/LICENSE.txt)

The repository includes ROMs compiled from the `uxnemu` reference
implementation, which are © Devine Lu Linvega and released under the MIT
license; see the [`roms/`](roms/) subfolder for details.
