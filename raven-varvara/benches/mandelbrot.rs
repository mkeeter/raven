use criterion::{Criterion, criterion_group, criterion_main};
use raven_varvara::Varvara;
use std::path::Path;
use uxn::{Backend, Uxn, UxnMem, backend};

fn load_rom() -> Vec<u8> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    let rom_path = Path::new(&manifest_dir)
        .parent()
        .expect("missing parent directory")
        .join("roms/mandelbrot.rom");
    std::fs::read(&rom_path).expect("could not read roms/mandelbrot.rom")
}

fn bench_startup<B: Backend>(c: &mut Criterion, rom: &[u8], name: &str) {
    c.bench_function(name, |b| {
        b.iter(|| {
            let mut mem = UxnMem::boxed();
            let mut vm = Uxn::<B>::new(&mut mem);
            let mut dev = Varvara::new();
            let remaining = vm.reset(rom);
            dev.reset(remaining);
            std::hint::black_box(vm.run(&mut dev, 0x100));
        });
    });
}

fn mandelbrot_benchmark(c: &mut Criterion) {
    let rom = load_rom();
    bench_startup::<backend::Interpreter>(c, &rom, "mandelbrot/interpreter");
    #[cfg(feature = "native")]
    bench_startup::<backend::Native>(c, &rom, "mandelbrot/native");
    #[cfg(feature = "tailcall")]
    bench_startup::<backend::Tailcall>(c, &rom, "mandelbrot/tailcall");
}

criterion_group!(benches, mandelbrot_benchmark);
criterion_main!(benches);
