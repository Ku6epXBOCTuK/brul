use std::hint::black_box;
use std::time::Duration;

use brul::{Component, Value, Variable, VariableId};
use brul_gui::{StyleSet, Widget, WidgetStyle};
use criterion::{Criterion, criterion_group, criterion_main};

const BINDINGS: usize = 6;
const NAMES: &[&str] = &["item", "position"];

fn component() -> Component {
    let mut component = Component::new();
    component.set_names(NAMES);
    component
}

fn item() -> Value {
    Value::Object(
        vec![
            ("icon".to_owned(), Value::String("folder".into())),
            ("name".to_owned(), Value::String("projects".into())),
            ("path".to_owned(), Value::String("C:/projects".into())),
            ("isDefault".to_owned(), Value::Bool(false)),
        ]
        .into(),
    )
}

fn push_row(component: &mut Component, position: f64) {
    component.push_build_scope(vec![
        Variable {
            name: VariableId::from_index(0),
            value: item(),
        },
        Variable {
            name: VariableId::from_index(1),
            value: Value::Number(position),
        },
    ]);
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
    let mut component = component();
    group.bench_function(format!("scopes_{BINDINGS}_bindings"), |b| {
        b.iter(|| {
            push_row(&mut component, 0.0);
            for _ in 0..BINDINGS {
                black_box(component.build_scopes());
            }
            component.pop_build_scope();
        });
    });
    group.bench_function("widgets", |b| {
        b.iter(|| black_box(row_widgets()));
    });
    group.finish();
}

fn build_scopes(c: &mut Criterion) {
    let mut group = group(c, "build_scopes");
    for depth in [1, 3] {
        let mut component = component();
        for level in 0..depth {
            push_row(&mut component, level as f64);
        }
        group.bench_function(format!("depth_{depth}"), |b| {
            b.iter(|| black_box(component.build_scopes()));
        });
    }
    group.finish();
}

fn apply_row(c: &mut Criterion) {
    let mut group = group(c, "apply_row");
    let mut scoped = component();
    push_row(&mut scoped, 0.0);
    let scopes = scoped.build_scopes();
    let mut component = component();
    group.bench_function("declares", |b| {
        b.iter(|| {
            component.push_binding_scopes(&scopes);
            component.pop_binding_scopes(scopes.len());
        });
    });
    group.finish();
}

criterion_group!(benches, each_row, build_scopes, apply_row);
criterion_main!(benches);
