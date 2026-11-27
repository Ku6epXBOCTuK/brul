use std::hint::black_box;
use std::time::Duration;

use brul::{Component, Object, Scope, Value, VariableId};
use criterion::{Criterion, criterion_group, criterion_main};

const NAMES: &[&str] = &["item", "position", "icon", "name", "path", "selected"];

type Row = (Value, f64);

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

fn row_scope() -> Scope {
    Scope::each((item(), 0.0_f64))
}

fn prop_component() -> Component {
    let mut component = Component::new();
    component.set_names(NAMES);
    component
        .declare_property(VariableId::from_index(2), Value::String("folder".into()))
        .unwrap();
    component
        .declare_property(VariableId::from_index(3), Value::String("projects".into()))
        .unwrap();
    component
        .declare_property(
            VariableId::from_index(4),
            Value::String("C:/projects".into()),
        )
        .unwrap();
    component
        .declare_property(VariableId::from_index(5), Value::Bool(false))
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
    group.bench_function("property_string_id", |b| {
        b.iter(|| black_box(property.read_id(VariableId::from_index(3)).unwrap()));
    });
    group.bench_function("property_scalar", |b| {
        b.iter(|| black_box(property.read("selected").unwrap()));
    });
    group.bench_function("property_scalar_id", |b| {
        b.iter(|| black_box(property.read_id(VariableId::from_index(5)).unwrap()));
    });
    let scope = row_scope();
    group.bench_function("each_scope_item", |b| {
        b.iter(|| black_box(scope.get::<Row>().unwrap().0.clone()));
    });
    group.bench_function("each_scope_position", |b| {
        b.iter(|| black_box(scope.get::<Row>().unwrap().1));
    });
    group.finish();
}

fn binding(c: &mut Criterion) {
    let mut group = group(c, "binding");
    let scope = row_scope();
    let mut component = prop_component();
    group.bench_function("props_read", |b| {
        b.iter(|| {
            component.set_scope(scope.clone());
            black_box(component.read("icon").unwrap());
            black_box(component.read("name").unwrap());
            black_box(component.read("path").unwrap());
        });
    });
    group.bench_function("props_read_id", |b| {
        b.iter(|| {
            component.set_scope(scope.clone());
            black_box(component.read_id(VariableId::from_index(2)).unwrap());
            black_box(component.read_id(VariableId::from_index(3)).unwrap());
            black_box(component.read_id(VariableId::from_index(4)).unwrap());
        });
    });
    group.bench_function("scope_swap", |b| {
        b.iter(|| {
            component.set_scope(scope.clone());
        });
    });
    group.finish();
}

criterion_group!(benches, read, binding);
criterion_main!(benches);
