use criterion::{criterion_group, criterion_main, Criterion};

fn bench_encode(_c: &mut Criterion) {
    // Will be implemented after encoder is working
}

criterion_group!(benches, bench_encode);
criterion_main!(benches);
