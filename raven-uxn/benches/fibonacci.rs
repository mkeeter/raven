use criterion::{Criterion, criterion_group, criterion_main};
use raven_uxn::{Backend, FIB, Uxn, UxnMem, backend};

fn bench_fib<B: Backend>(c: &mut Criterion, name: &str) {
    c.bench_function(name, |b| {
        b.iter(|| {
            let mut ram = UxnMem::boxed();
            let mut vm = Uxn::<B>::new(&mut ram);
            let mut dev = raven_uxn::EmptyDevice;
            let _ = vm.reset(FIB);
            vm.ram_write_byte(0x101, 23);
            std::hint::black_box(vm.run(&mut dev, 0x100));
        });
    });
}

fn fibonacci_benchmark(c: &mut Criterion) {
    bench_fib::<backend::Interpreter>(c, "fibonacci/interpreter");
    #[cfg(feature = "native")]
    bench_fib::<backend::Native>(c, "fibonacci/native");
    #[cfg(feature = "tailcall")]
    bench_fib::<backend::Tailcall>(c, "fibonacci/tailcall");
}

criterion_group!(benches, fibonacci_benchmark);
criterion_main!(benches);
