**Raven** is an independent re-implementation of the
[Uxn CPU](https://wiki.xxiivv.com/site/uxn.html)
and
[Varvara Ordinator](https://wiki.xxiivv.com/site/varvara.html).

For details, see [the project writeup](https://mattkeeter.com/projects/raven).

--------------------------------------------------------------------------------

The `raven-uxn` crate includes three backends for Uxn CPU emulation:

- The baseline interpreter is a `#[no_std]` crate written in 100% safe Rust, with a
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
- The tailcall interpreter is also written in 100% safe Rust, but requires
  running a nightly toolchain.  It is the fastest backend on AArch64, and faster
  than the baseline interpreter on `x86-64` (but slower than the native assembly
  implementation)

Backends are feature-gated because they have platform and toolchain
requirements; when making a desktop build, use `--all-features` to select all
backends.

The native interpreter can be checked against the baseline interpreter with fuzz
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

It's also possible to compile `raven-cli` as a WASM application (for use with
[`wasmtime`](https://github.com/bytecodealliance/wasmtime) or similar):

```console
rustup +nightly target add wasm32-wasip1
cargo +nightly build --release -praven-cli --target=wasm32-wasip1
# add `--features=tailcall` to test tailcall performance in WASM
```

--------------------------------------------------------------------------------

© 2024-2026 Matthew Keeter  
Released under the [Mozilla Public License 2.0](https://github.com/mkeeter/fidget/blob/main/LICENSE.txt)

The repository includes ROMs compiled from the `uxnemu` reference
implementation, which are © Devine Lu Linvega and released under the MIT
license; see the [`roms/`](roms/) subfolder for details.
