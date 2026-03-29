# 0.3.0
TODO write stuff here

# 0.2.1
- Fix bug in initial Varvara screen buffer allocation
- Bump dependencies to latest releases
- Update to Rust 2024 edition

# 0.2.0
- Add x86-64 interpreter backend, which is up to 2× faster
- Enable `native` feature by default for `raven-uxn` and `raven-varvara`
    - Users of the crates can disable this feature if necessary (e.g. for the
      web platform)
    - The end-user applications (`raven-cli` and `raven-gui`) only enable the
      `native` feature on appropriate platforms

# 0.1.0
Initial release
