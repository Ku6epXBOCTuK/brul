use std::hint::black_box;
use std::time::Duration;

use brul::{Component, Value, Variable, VariableId};
use criterion::{Criterion, criterion_group, criterion_main};

const NAMES: &[&str] = &["item", "position", "icon", "name", "path", "selected"];

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

fn row_scopes() -> std::rc::Rc<[Vec<Variable>]> {
    let mut scoped = Component::new();
    scoped.set_names(NAMES);
    scoped.push_build_scope(&[
        Variable {
            name: VariableId::from_index(0),
            value: item(),
        },
        Variable {
            name: VariableId::from_index(1),
            value: Value::Number(0.0),
        },
    ]);
    scoped.build_scopes()
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
    let scopes = row_scopes();
    let mut frame = Component::new();
    frame.set_names(NAMES);
    frame.push_binding_scopes(&scopes);
    group.bench_function("variable_in_frame", |b| {
        b.iter(|| black_box(frame.read("position").unwrap()));
    });
    group.bench_function("variable_in_frame_id", |b| {
        b.iter(|| black_box(frame.read_id(VariableId::from_index(1)).unwrap()));
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
    group.bench_function("props_read_id", |b| {
        b.iter(|| {
            component.push_binding_scopes(&scopes);
            black_box(component.read_id(VariableId::from_index(2)).unwrap());
            black_box(component.read_id(VariableId::from_index(3)).unwrap());
            black_box(component.read_id(VariableId::from_index(4)).unwrap());
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
