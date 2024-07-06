**Raven** is an independent re-implementation of the
[Uxn CPU](https://wiki.xxiivv.com/site/uxn.html)
and
[Varvara Ordinator](https://wiki.xxiivv.com/site/varvara.html).

For details, see [the project writeup](https://mattkeeter.com/projects/raven).

--------------------------------------------------------------------------------

The `raven-uxn` crate implements two implementations of the Uxn CPU:

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

--------------------------------------------------------------------------------

The Varvara implementation (`raven-varvara`) includes all peripherals, and has
been tested on many of the
[flagship applications](https://wiki.xxiivv.com/site/roms.html)
(Left, Orca, Noodle, Potato).
