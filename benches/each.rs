use std::hint::black_box;
use std::time::Duration;

use brul::{Component, Object, Scope, Value};
use brul_gui::{StyleSet, Widget, WidgetStyle};
use criterion::{Criterion, criterion_group, criterion_main};

const BINDINGS: usize = 6;
const NAMES: &[&str] = &["item", "position"];

type Row = (Value, f64);

fn component() -> Component {
    let mut component = Component::new();
    component.set_names(NAMES);
    component
}

fn item() -> Value {
    Value::Object(
        Object::builder()
            .field("icon", Value::String("folder".into()))
            .field("name", Value::String("projects".into()))
            .field("path", Value::String("C:/projects".into()))
            .field("isDefault", Value::Bool(false))
            .into(),
    )
}

fn push_row(component: &mut Component, position: f64) {
    component.set_scope(Scope::each((item(), position)));
}

fn row_widgets() -> Widget {
    Widget::row(vec![
        Widget::text("folder"),
        Widget::column(vec![Widget::text("projects"), Widget::text("C:/projects")]),
    ])
    .with_styles(StyleSet::new(WidgetStyle::default()))
}

fn group<'a>(
    c: &'a mut Criterion,
    name: &str,
) -> criterion::BenchmarkGroup<'a, criterion::measurement::WallTime> {
    let mut group = c.benchmark_group(name);
    group
        .sample_size(50)
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2));
    group
}

fn each_row(c: &mut Criterion) {
    let mut group = group(c, "each_row");
    let mut row = component();
    group.bench_function(format!("scope_{BINDINGS}_bindings"), |b| {
        b.iter(|| {
            push_row(&mut row, 0.0);
            for _ in 0..BINDINGS {
                black_box(row.scope().get::<Row>().unwrap().1);
            }
        });
    });
    group.bench_function("widgets", |b| {
        b.iter(|| black_box(row_widgets()));
    });
    group.finish();
}

fn scope_depth(c: &mut Criterion) {
    let mut group = group(c, "scope_depth");
    for depth in [1, 3] {
        let mut component = component();
        for level in 0..depth {
            push_row(&mut component, level as f64);
        }
        group.bench_function(format!("depth_{depth}"), |b| {
            b.iter(|| black_box(component.scope().clone()));
        });
    }
    group.finish();
}

fn apply_row(c: &mut Criterion) {
    let mut group = group(c, "apply_row");
    let mut scoped = component();
    push_row(&mut scoped, 0.0);
    let scope = scoped.scope().clone();
    let mut component = component();
    group.bench_function("scope_swap", |b| {
        b.iter(|| {
            component.set_scope(scope.clone());
        });
    });
    group.finish();
}

criterion_group!(benches, each_row, scope_depth, apply_row);
criterion_main!(benches);
