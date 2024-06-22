# Uxn
The Uxn CPU is implemented in the `raven-uxn` crate.

This implementation is focused on speed, safety, and correctness.

It is written in Rust as a `#[no_std]` crate, so it can used as part of a
bare-metal system.  A `#![forbid(unsafe_code)]` annotation ensures that the
crate only uses safe Rust, and the crate has only one dependency (
[`zerocopy`](https://https://crates.io/crates/zerocopy)).

## Performance
`raven-uxn` is typically 10-20% faster for CPU-heavy workloads than the
reference implementation
([`uxnemu`](https://git.sr.ht/~rabbits/uxn/tree/main/item/src)).

Running the
[`mandelbrot.tal` demo](https://git.sr.ht/~rabbits/uxn/tree/main/item/projects/examples/demos/mandelbrot.tal)
at max scale (`#0020`), `raven-gui` is about 20% faster than the `uxnemu`
reference implementation: it calculates the fractal in 1.57 seconds, versus 2.03
seconds for `uxnemu`.

Calculating the first 35 Fibonnaci numbers using
[`fib.tal`](https://git.sr.ht/~rabbits/uxn/tree/main/item/projects/examples/exercises/fib.tal),
`raven-cli` takes 1.44 seconds (versus 1.65 seconds for `uxnemu`).

## Design
The Uxn processor has 256 instructions.  This sounds like a lot, but – compared
to a register machine – it's very few possibilities!

`raven-uxn` implements each of the 256 instructions as functions, then runs a
tight loop that dispatches based on opcode.  _Everything_ is inlined, so
`Uxn::run` ends up being a single gigantic (11.4 KiB) function; this sounds like
a lot, but it's only an average of 11 instructions per opcode.  Pervasive
inlining means that all of our important data – stack pointers, offsets, etc –
can be kept in registers, making the evaluation loop very efficient.

The assembly is also hand-inspected for inefficiency and panics; `Uxn::run`
currently has no panicking paths.

# Varvara
## Design
The `raven-varvara` crate is independent of any specific GUI / windowing
implementation.  Instead, the application _using_ the crate is responsible for
running the event loop, sending keyboard / mouse state, and drawing the returned
frames.  This makes the library very flexible!

## Devices
### Console
#### Limitations
Output streams are buffered and printing is delegated to the caller.  For
example, a program that prints many lines before halting will run to completion,
_then_ the caller is responsible for printing those lines

### Audio
#### Implementation notes
The [reference implementation](https://git.sr.ht/~rabbits/uxn/tree/main/item/src/devices/audio.c)
is very different from the
[specification](https://wiki.xxiivv.com/site/varvara.html#audio);
`raven-varvara` attempt to match the behavior of the reference implementation.

### Controller
#### Implementation notes
The `key` port **must** be cleared after the vector is called.  Otherwise,
button handling is broken in some ROMs.

### File
#### Implementation notes
The directory output format must be zero-terminated; otherwise, the Potato ROM
prints junk data left in memory.

### Datetime
#### Limitations
The `IS_DST` bit always returns 0
(see [`chrono#1562`](https://github.com/chronotope/chrono/issues/1562))
