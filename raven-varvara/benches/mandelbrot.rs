use criterion::{Criterion, criterion_group, criterion_main};
use raven_varvara::Varvara;
use std::path::Path;
use uxn::{Backend, Uxn, UxnMem};

fn load_rom() -> Vec<u8> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    let rom_path = Path::new(&manifest_dir)
        .parent()
        .expect("missing parent directory")
        .join("roms/mandelbrot.rom");
    std::fs::read(&rom_path).expect("could not read roms/mandelbrot.rom")
}

fn bench_startup(c: &mut Criterion, rom: &[u8], backend: Backend, name: &str) {
    c.bench_function(name, |b| {
        b.iter(|| {
            let mut mem = UxnMem::boxed();
            let mut vm = Uxn::new(&mut mem);
            let mut dev = Varvara::new();
            let remaining = vm.reset(rom);
            dev.reset(remaining);
            std::hint::black_box(vm.run(&mut dev, 0x100, backend));
        });
    });
}

fn mandelbrot_benchmark(c: &mut Criterion) {
    let rom = load_rom();
    bench_startup(c, &rom, Backend::Interpreter, "mandelbrot/interpreter");
    #[cfg(feature = "native")]
    bench_startup(c, &rom, Backend::Native, "mandelbrot/native");
}

criterion_group!(benches, mandelbrot_benchmark);
criterion_main!(benches);
