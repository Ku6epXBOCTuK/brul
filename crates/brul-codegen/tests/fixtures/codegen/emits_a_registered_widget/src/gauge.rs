#[brul::widget(name = "Gauge", events(changed: [Number]))]
pub fn gauge(
    _theme: Option<&str>,
    _build: &mut brul::Build,
    _component: &brul::ComponentHandle,
    value: f64,
    label: String,
) -> Result<brul::Built, brul::ViewError> {
    let widget = brul::gui::Widget::text(format!("{label}: {value}"));
    Ok(brul::Built {
        widget,
        bindings: Vec::new(),
        handlers: Vec::new(),
        props: Vec::new(),
    })
}
