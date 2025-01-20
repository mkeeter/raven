**Raven** is an independent re-implementation of the
[Uxn CPU](https://wiki.xxiivv.com/site/uxn.html)
and
[Varvara Ordinator](https://wiki.xxiivv.com/site/varvara.html).

For details, see [the project writeup](https://mattkeeter.com/projects/raven).

--------------------------------------------------------------------------------

The `raven-uxn` crate includes two implementations of the Uxn CPU:

- The safe interpreter is a `#[no_std]` crate written in 100% safe Rust, with a
  single dependency (`zerocopy`).  It is 10-20% faster than
  the [reference implementation](https://git.sr.ht/~rabbits/uxn/tree/main/item/src)
  for CPU-heavy workloads (e.g.
  [`fib.tal`](https://git.sr.ht/~rabbits/uxn/tree/main/item/projects/examples/exercises/fib.tal),
  and
  [`mandelbrot.tal`](https://git.sr.ht/~rabbits/uxn/tree/main/item/projects/examples/demos/mandelbrot.tal)
- The unsafe ("native") interpreter is written in `aarch64` assembly (with Rust
  shims on either side), and runs 40-50% faster than the reference
  implementation

The native interpreter can be checked against the safe interpreter with fuzz
testing:

```console
cargo install cargo-fuzz # this only needs to be run once
cargo +nightly fuzz run --release fuzz-native
```

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

The web demo is built with [`truck`](https://trunkrs.dev/), e.g.

```console
cargo install --locked trunk # this only needs to be run once
cd raven-gui
trunk build --release --public-url=/projects/raven/demo/ # edit this path
```

--------------------------------------------------------------------------------

© 2024-2025 Matthew Keeter  
Released under the [Mozilla Public License 2.0](https://github.com/mkeeter/fidget/blob/main/LICENSE.txt)

The repository includes ROMs compiled from the `uxnemu` reference
implementation, which are © Devine Lu Linvega and released under the MIT
license; see the [`roms/`](roms/) subfolder for details.
