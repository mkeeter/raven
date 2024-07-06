# Design
The `raven-varvara` crate is independent of any specific GUI / windowing
implementation.  Instead, the application _using_ the crate is responsible for
running the event loop, sending keyboard / mouse state, and drawing the returned
frames.  This makes the library very flexible!

# Devices
## Console
### Limitations
Output streams are buffered and printing is delegated to the caller.  For
example, a program that prints many lines before halting will run to completion,
_then_ the caller is responsible for printing those lines

## Audio
### Implementation notes
The [reference implementation](https://git.sr.ht/~rabbits/uxn/tree/main/item/src/devices/audio.c)
is very different from the
[specification](https://wiki.xxiivv.com/site/varvara.html#audio);
`raven-varvara` attempt to match the behavior of the reference implementation.

## Controller
### Implementation notes
The `key` port **must** be cleared after the vector is called.  Otherwise,
button handling is broken in some ROMs.

## File
### Implementation notes
The directory output format must be zero-terminated; otherwise, the Potato ROM
prints junk data left in memory.

## Datetime
### Limitations
The `IS_DST` bit always returns 0
(see [`chrono#1562`](https://github.com/chronotope/chrono/issues/1562))
