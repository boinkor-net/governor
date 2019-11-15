use criterion::{criterion_group, criterion_main};

mod multi_threaded;
mod single_threaded;

criterion_group!(
    benches,
    single_threaded::bench_all,
    multi_threaded::bench_all
);
criterion_main!(benches);
