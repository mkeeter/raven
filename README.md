**Cardinal** is a fork of Raven an independent re-implementation of the
[Uxn CPU](https://wiki.xxiivv.com/site/uxn.html)
and
[Varvara Ordinator](https://wiki.xxiivv.com/site/varvara.html).


The Uxn/Varvara ecosystem is a **personal computing stack**.

Cardinal is my personal stack for the Uxn/Varvara ecosystem.

For details on project origins, see [Raven's project writeup](https://mattkeeter.com/projects/cardinal).

--------------------------------------------------------------------------------

The `cardinal-uxn` crate includes two implementations of the Uxn CPU:

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

The Varvara implementation (`cardinal-varvara`) includes all peripherals, and has
been tested on many of the
[flagship applications](https://wiki.xxiivv.com/site/roms.html)
(Left, Orca, Noodle, Potato).

--------------------------------------------------------------------------------

The repository includes two applications built on these libraries:

- `cardinal-cli` is a command-line application to run console-based ROMs
- `cardinal-gui` is a full-fledged GUI, which runs both as a native application and
  [Raven on the web](https://mattkeeter.com/projects/cardinal/demo)

The web demo is built with [`truck`](https://trunkrs.dev/), e.g.

```console
cargo install --locked trunk # this only needs to be run once
cd cardinal-gui
trunk build --release --public-url=/projects/cardinal/demo/ # edit this path
```

--------------------------------------------------------------------------------

July 2025 Changes
- stdout and stderr callbacks

© 2024-2025 Matthew Keeter, David Horner
Released under the [Mozilla Public License 2.0](/LICENSE.txt)

The repository includes ROMs compiled from the `uxnemu` reference
implementation, which are © Devine Lu Linvega and released under the MIT
license; see the [`roms/`](roms/) subfolder for details.
