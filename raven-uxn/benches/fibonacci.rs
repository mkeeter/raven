use criterion::{Criterion, criterion_group, criterion_main};
use raven_uxn::{Backend, FIB, Uxn, UxnMem};

fn bench_fib(c: &mut Criterion, backend: Backend, name: &str) {
    c.bench_function(name, |b| {
        b.iter(|| {
            let mut ram = UxnMem::boxed();
            let mut vm = Uxn::new(&mut ram);
            let mut dev = raven_uxn::EmptyDevice;
            let _ = vm.reset(FIB);
            vm.ram_write_byte(0x101, 23);
            std::hint::black_box(vm.run(&mut dev, 0x100, backend));
        });
    });
}

fn fibonacci_benchmark(c: &mut Criterion) {
    bench_fib(c, Backend::Interpreter, "fibonacci/interpreter");
    #[cfg(feature = "native")]
    bench_fib(c, Backend::Native, "fibonacci/native");
    #[cfg(feature = "tailcall")]
    bench_fib(c, Backend::Tailcall, "fibonacci/tailcall");
}

criterion_group!(benches, fibonacci_benchmark);
criterion_main!(benches);
