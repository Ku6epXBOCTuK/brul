use std::hint::black_box;
use std::time::Duration;

use brul::{Component, Value, Variable};
use criterion::{Criterion, criterion_group, criterion_main};

fn item() -> Value {
    Value::Object(vec![
        ("icon".to_owned(), Value::String("folder".to_owned())),
        ("name".to_owned(), Value::String("projects".to_owned())),
        ("path".to_owned(), Value::String("C:/projects".to_owned())),
        ("isDefault".to_owned(), Value::Bool(false)),
    ])
}

fn row_scopes() -> std::rc::Rc<[Vec<Variable>]> {
    let mut scoped = Component::new();
    scoped.push_build_scope(&[
        Variable {
            name: "item",
            value: item(),
        },
        Variable {
            name: "position",
            value: Value::Number(0.0),
        },
    ]);
    scoped.build_scopes()
}

fn prop_component() -> Component {
    let mut component = Component::new();
    component
        .declare_property("icon", Value::String("folder".to_owned()))
        .unwrap();
    component
        .declare_property("name", Value::String("projects".to_owned()))
        .unwrap();
    component
        .declare_property("path", Value::String("C:/projects".to_owned()))
        .unwrap();
    component
        .declare_property("selected", Value::Bool(false))
        .unwrap();
    component
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

fn read(c: &mut Criterion) {
    let mut group = group(c, "read");
    let property = prop_component();
    group.bench_function("property_string", |b| {
        b.iter(|| black_box(property.read("name").unwrap()));
    });
    group.bench_function("property_scalar", |b| {
        b.iter(|| black_box(property.read("selected").unwrap()));
    });
    let scopes = row_scopes();
    let mut frame = Component::new();
    frame.push_binding_scopes(&scopes);
    group.bench_function("variable_in_frame", |b| {
        b.iter(|| black_box(frame.read("position").unwrap()));
    });
    group.finish();
}

fn binding(c: &mut Criterion) {
    let mut group = group(c, "binding");
    let scopes = row_scopes();
    let mut component = prop_component();
    group.bench_function("props_read", |b| {
        b.iter(|| {
            component.push_binding_scopes(&scopes);
            black_box(component.read("icon").unwrap());
            black_box(component.read("name").unwrap());
            black_box(component.read("path").unwrap());
            component.pop_binding_scopes(scopes.len());
        });
    });
    group.bench_function("frame_only", |b| {
        b.iter(|| {
            component.push_binding_scopes(&scopes);
            component.pop_binding_scopes(scopes.len());
        });
    });
    group.finish();
}

criterion_group!(benches, read, binding);
criterion_main!(benches);
