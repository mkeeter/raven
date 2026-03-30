_default:
    just --list --unsorted

GUI_DIR  := "raven-gui"
PKG_DIR  := GUI_DIR / "pkg"
PKG_JS   := PKG_DIR / "raven-gui.js"
PKG_WA   := PKG_DIR / "raven-gui.opt.wasm"
DIST_DIR := GUI_DIR / "dist"
CUT8 := ".{56}$" # regex to strip the trailing 56 characters

# Run `raven-cli` with all features
cli +ARGS:
    cargo +nightly run -praven-cli --release --all-features -- {{ARGS}}

# Run `raven-gui` with all features
gui +ARGS:
    cargo +nightly run -praven-gui --release --all-features -- {{ARGS}}

# Run `cargo check` with all features
check:
    cargo +nightly check --all-features --all-targets
    cargo +nightly check --features=tailcall --target wasm32-unknown-unknown

# Run `cargo test` with all features
test:
    cargo +nightly test --release --all-features --all-targets

# Run `cargo fuzz` to test the native implementation
fuzz:
    cargo +nightly fuzz run -O fuzz-native --

# Run benchmarks
bench:
    cargo +nightly bench --all-features

# Build a web application in `raven-gui/dist`
dist:
    rustup +nightly target add wasm32-unknown-unknown
    cargo +nightly build --release -praven-gui --features=tailcall --target wasm32-unknown-unknown
    wasm-bindgen target/wasm32-unknown-unknown/release/raven-gui.wasm --out-dir {{PKG_DIR}} --target web
    wasm-opt -O {{PKG_DIR}}/raven-gui_bg.wasm -o {{PKG_WA}}
    mkdir -p  {{DIST_DIR}}
    rm    -rf {{DIST_DIR}}/*
    cp {{PKG_WA}} '{{DIST_DIR}}/raven-gui.{{replace_regex(sha256_file(PKG_WA), CUT8, "")}}.wasm'
    cp {{PKG_JS}} '{{DIST_DIR}}/raven-gui.{{replace_regex(sha256_file(PKG_JS), CUT8, "")}}.js'
    cat {{GUI_DIR}}/index.html \
        | sed s/JSHASH/{{replace_regex(sha256_file(PKG_JS), CUT8, "")}}/g \
        | sed s/WAHASH/{{replace_regex(sha256_file(PKG_WA), CUT8, "")}}/g \
        > {{DIST_DIR}}/index.html

# Build and serve the web application
serve port="8000":
    just dist
    cd {{DIST_DIR}} && python3 -m http.server {{port}}

# Deploy the demo to `mattkeeter.com/projects/raven/demo`
deploy:
    just dist
    rsync -avz --delete -e ssh {{DIST_DIR}}/ mkeeter@mattkeeter.com:mattkeeter.com/projects/raven/demo/
