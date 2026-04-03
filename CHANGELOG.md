# 0.3.0
This release has many breaking API changes in support of a tailcall interpreter.
The tailcall interpreter is faster than the baseline interpreter across both
ARM64 and x86-64 backends, and also beats the native assembly backend on ARM64.

- The Uxn CPU now borrows **all** of its bulk memory (both RAM and stacks);
  previously, it only borrowed RAM.  The `UxnRam` type has been removed, and
  there is a new `UxnMem` type which serves the same purpose.
- The `struct Uxn` is now parameterized with a `Backend` template parameter,
  instead of accepting it as an argument.  Available backends are in the
  `raven_uxn::backend` module.
- All backends except `backend::Interpreter` are now feature-gated.  This rolls
  back changes made in 0.2.0, which enabled the `native` feature was enabled by
  default.  Choose wisely!
- The Uxn CPU is split into a top-level `struxt Uxn<Backend>` which wraps an
  inner `struct UxnCore`.  The `deo` and `dei` functions in `trait Device` now
  take a `&mut UxnCore`.  This is necessary for fiddly Rust reasons; see the
  `UxnCore` docstring for details.
- `raven-gui` and `raven-cli` now accept a `--backend` argument to select
  between `interpreter`, `native`, and `tailcall` backends.
- `raven-cli` now accepts a `-q` argument to quit after initialization, for
  unscientific benchmarking.

## Other small changes
- Updated to Rust 2024
- Web demo is now deployed without Trunk
- Repo includes a Justfile to run common tasks
- `raven-gui` prints its startup time
- Fix bug in initial Varvara screen buffer allocation
- Bump dependencies to latest releases

# 0.2.0
- Add x86-64 interpreter backend, which is up to 2× faster
- Enable `native` feature by default for `raven-uxn` and `raven-varvara`
    - Users of the crates can disable this feature if necessary (e.g. for the
      web platform)
    - The end-user applications (`raven-cli` and `raven-gui`) only enable the
      `native` feature on appropriate platforms

# 0.1.0
Initial release
