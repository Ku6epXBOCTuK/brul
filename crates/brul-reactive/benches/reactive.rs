use std::hint::black_box;
use std::time::Duration;

use brul_reactive::{Effect, Memo, Signal, batch, flush};
use criterion::{Criterion, criterion_group, criterion_main};

fn chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("chain");
    group
        .sample_size(50)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));

    for size in [10, 100, 1000] {
        let source = Signal::new(0i32);
        let mut last = Memo::new(move || source.get());
        for _ in 1..size {
            let previous = last;
            last = Memo::new(move || previous.get() + 1);
        }
        let _effect = Effect::new(move || {
            black_box(last.get());
        });
        let mut value = 0i32;
        group.bench_function(format!("{size}"), |b| {
            b.iter(|| {
                value = value.wrapping_add(1);
                source.set(black_box(value));
                flush();
            });
        });
    }

    group.finish();
}

fn diamond(c: &mut Criterion) {
    let mut group = c.benchmark_group("diamond");
    group
        .sample_size(50)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));

    let source = Signal::new(0i32);
    let left = Memo::new(move || source.get() + 1);
    let right = Memo::new(move || source.get() * 2);
    let joined = Memo::new(move || left.get() + right.get());
    let _effect = Effect::new(move || {
        black_box(joined.get());
    });
    let mut value = 0i32;

    group.bench_function("join", |b| {
        b.iter(|| {
            value = value.wrapping_add(1);
            source.set(black_box(value));
            flush();
        });
    });

    group.finish();
}

fn many_subscribers(c: &mut Criterion) {
    let mut group = c.benchmark_group("many_subscribers");
    group
        .sample_size(50)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));

    for size in [10, 100, 1000] {
        let source = Signal::new(0i32);
        let _effects: Vec<Effect> = (0..size)
            .map(|_| {
                Effect::new(move || {
                    black_box(source.get());
                })
            })
            .collect();
        let mut value = 0i32;
        group.bench_function(format!("{size}"), |b| {
            b.iter(|| {
                value = value.wrapping_add(1);
                source.set(black_box(value));
                flush();
            });
        });
    }

    group.finish();
}

fn batched_writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("batched_writes");
    group
        .sample_size(50)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));

    for size in [10, 100] {
        let signals: Vec<Signal<i32>> = (0..size).map(Signal::new).collect();
        let read = signals.clone();
        let _effect = Effect::new(move || {
            for signal in &read {
                black_box(signal.get());
            }
        });
        let mut value = 0i32;
        group.bench_function(format!("{size}"), |b| {
            b.iter(|| {
                value = value.wrapping_add(1);
                batch(|| {
                    for signal in &signals {
                        signal.set(black_box(value));
                    }
                });
            });
        });
    }

    group.finish();
}

criterion_group!(benches, chain, diamond, many_subscribers, batched_writes);
criterion_main!(benches);
