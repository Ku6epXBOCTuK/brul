use super::*;
use std::cell::RefCell;
use std::rc::Rc;

use brul::gui::{KeyModifiers, KeyPress, Widget, WidgetTree};
use brul::{
    Component, ComponentHandle, Dyn, EditBuffer, EventTarget, SharedStates, StateId, States,
};

struct Rig {
    tree: Rc<RefCell<WidgetTree>>,
    ctx: Build,
    state: StateId,
}

fn rig(widget: Widget, target: WidgetId, state: StateId) -> Rig {
    let tree = Rc::new(RefCell::new(WidgetTree::new(widget)));
    let states: SharedStates = Rc::new(RefCell::new(States::default()));
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut ctx = Build::new();
    ctx.set_tree(Rc::clone(&tree));
    ctx.set_states(states);
    ctx.set_target(EventTarget::Widget(target), state, component);
    Rig { tree, ctx, state }
}

fn buffer(rig: &Rig, id: WidgetId) -> EditBuffer {
    rig.tree.borrow().edit_buffer(id).expect("input buffer")
}

fn key(id: WidgetId, code: &str) -> RawUiEvent {
    RawUiEvent::widget(
        id,
        RawEvent::key(KeyPress::from(code), KeyModifiers::default()),
    )
}

fn apply(rig: &mut Rig, value: &str, mode: &str) {
    __brul_attach_masked_editor_apply(
        &mut rig.ctx,
        rig.state,
        &[Dyn::text(value), Dyn::text(mode)],
    )
    .expect("apply");
}

fn type_text(rig: &mut Rig, id: WidgetId, text: &str) {
    for character in text.chars() {
        let dispatch = __brul_attach_masked_editor_raw_event(
            &mut rig.ctx,
            rig.state,
            &key(id, &character.to_string()),
        );
        assert!(dispatch.is_handled());
    }
}

fn editor_state(rig: &Rig) -> Rc<RefCell<Editor>> {
    rig.ctx.get_state::<Editor>().expect("editor state")
}

#[test]
fn generated_view_registers_both_policies() {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let mut state = component.borrow_mut();
    state.set_names(generated::names());
    state.set_state(generated::CustomInputScriptState {
        secret: brul::Signal::new(String::new()),
        mode: brul::Signal::new("mask".to_owned()),
        caret: brul::Signal::new(String::new()),
        status: brul::Signal::new("mask".to_owned()),
    });
    drop(state);

    let mut build = Build::new();
    generated::custom_input_view(Some(generated::ThemeId::Dark), &mut build, &component)
        .expect("generated view");
    let capabilities = build.take_capabilities();
    type Apply = fn(&mut Build, StateId, &[Dyn]) -> Result<(), ViewError>;
    let masked: Apply = __brul_attach_masked_editor_apply;
    let keys: Apply = __brul_attach_custom_keys_apply;
    assert!(
        capabilities
            .iter()
            .any(|hooks| std::ptr::fn_addr_eq(hooks.apply, masked))
    );
    assert!(
        capabilities
            .iter()
            .any(|hooks| std::ptr::fn_addr_eq(hooks.apply, keys))
    );
}

#[test]
fn mask_hides_the_value_and_maps_the_caret() {
    let input = Widget::input("");
    let id = input.id();
    let mut rig = rig(input, id, StateId::source("custom_input", "mask"));
    rig.tree.borrow_mut().set_focused(Some(id));
    apply(&mut rig, "", "mask");

    type_text(&mut rig, id, "ab");

    let buffer = buffer(&rig, id);
    assert_eq!(buffer.text(), "\u{2022}\u{2022}");
    assert_eq!(buffer.caret().offset, 6);
    drop(buffer);
    assert_eq!(editor_state(&rig).borrow().raw, "ab");
}

#[test]
fn reverse_maps_display_caret_back_to_the_raw_caret() {
    let input = Widget::input("");
    let id = input.id();
    let mut rig = rig(input, id, StateId::source("custom_input", "reverse"));
    rig.tree.borrow_mut().set_focused(Some(id));
    apply(&mut rig, "", "reverse");

    type_text(&mut rig, id, "ab");
    assert_eq!(buffer(&rig, id).text(), "ba");
    assert_eq!(buffer(&rig, id).caret().offset, 0);

    let dispatch =
        __brul_attach_masked_editor_raw_event(&mut rig.ctx, rig.state, &key(id, "ArrowLeft"));
    assert!(dispatch.is_handled());
    assert_eq!(buffer(&rig, id).caret().offset, 1);
    assert_eq!(editor_state(&rig).borrow().caret, 1);
}

#[test]
fn digest_never_writes_the_raw_value_to_the_buffer() {
    let input = Widget::input("");
    let id = input.id();
    let mut rig = rig(input, id, StateId::source("custom_input", "digest"));
    rig.tree.borrow_mut().set_focused(Some(id));
    apply(&mut rig, "", "digest");

    type_text(&mut rig, id, "a");

    let buffer = buffer(&rig, id);
    let display = buffer.text();
    assert_eq!(display.len(), 16);
    assert!(
        display
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    );
    drop(buffer);
    assert_eq!(editor_state(&rig).borrow().raw, "a");
}

#[test]
fn custom_keys_override_tab_and_escape() {
    let input = Widget::input("");
    let other = Widget::input("");
    let focusables = [input.id(), other.id()];
    let mut rig = rig(
        Widget::row(vec![input, other]),
        focusables[0],
        StateId::source("custom_input", "keys"),
    );
    rig.tree.borrow_mut().set_focused(Some(focusables[0]));
    __brul_attach_custom_keys_apply(&mut rig.ctx, rig.state, &[]).expect("apply");

    let dispatch =
        __brul_attach_custom_keys_raw_event(&mut rig.ctx, rig.state, &key(focusables[0], "Tab"));
    assert!(dispatch.stops_capabilities());
    assert_eq!(rig.tree.borrow().focused_id(), Some(focusables[1]));

    let dispatch =
        __brul_attach_custom_keys_raw_event(&mut rig.ctx, rig.state, &key(focusables[1], "Escape"));
    assert!(dispatch.stops_capabilities());
    assert_eq!(rig.tree.borrow().focused_id(), None);
}
