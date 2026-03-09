use criterion::{criterion_group, criterion_main, Criterion};
use raven_uxn::{Backend, Uxn, UxnRam, FIB};

fn bench_fib(c: &mut Criterion, backend: Backend, name: &str) {
    c.bench_function(name, |b| {
        b.iter(|| {
            let mut ram = UxnRam::new();
            let mut vm = Uxn::new(&mut ram, backend);
            let mut dev = raven_uxn::EmptyDevice;
            let _ = vm.reset(FIB);
            vm.ram_write_byte(0x101, 23);
            std::hint::black_box(vm.run(&mut dev, 0x100));
        });
    });
}

fn fibonacci_benchmark(c: &mut Criterion) {
    bench_fib(c, Backend::Interpreter, "fibonacci/interpreter");
    #[cfg(feature = "native")]
    bench_fib(c, Backend::Native, "fibonacci/native");
}

criterion_group!(benches, fibonacci_benchmark);
criterion_main!(benches);
