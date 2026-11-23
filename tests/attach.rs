use std::cell::RefCell;
use std::rc::Rc;

use brul::{
    Build, Component, ComponentHandle, Dyn, Event, SharedStates, StateId, States, UiEvent,
    ViewError,
};

thread_local! {
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug, PartialEq)]
struct CounterState {
    amount: f64,
    flag: bool,
    label: String,
}

#[brul::attach(
    name = "counter",
    events(Key),
    update = counter_update,
    event = counter_event,
    cleanup = counter_cleanup
)]
fn counter(
    _ctx: &mut Build,
    amount: f64,
    flag: bool,
    label: String,
) -> Result<CounterState, ViewError> {
    LOG.with(|log| log.borrow_mut().push("create".into()));
    Ok(CounterState {
        amount,
        flag,
        label,
    })
}

fn counter_update(
    state: &mut CounterState,
    _ctx: &mut Build,
    amount: f64,
    flag: bool,
    label: String,
) -> Result<(), ViewError> {
    LOG.with(|log| log.borrow_mut().push(format!("{amount}|{flag}|{label}")));
    state.amount = amount;
    state.flag = flag;
    state.label = label;
    Ok(())
}

fn counter_event(_state: &mut CounterState, _ctx: &mut Build, event: &UiEvent) -> bool {
    matches!(event.event, Event::Key { .. })
}

fn counter_cleanup(state: &mut CounterState, _ctx: &mut Build) {
    LOG.with(|log| log.borrow_mut().push(format!("cleanup:{}", state.amount)));
}

#[test]
fn attach_shims_create_update_event_and_cleanup_state() {
    LOG.with(|log| log.borrow_mut().clear());

    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let tree = Rc::new(RefCell::new(brul::gui::WidgetTree::new(
        brul::gui::Widget::text("root"),
    )));
    let states: SharedStates = Rc::new(RefCell::new(States::default()));
    let state_id = StateId::new(42);
    let mut ctx = Build::new();
    ctx.set_tree(tree);
    ctx.set_states(Rc::clone(&states));
    ctx.set_target(brul::gui::WidgetId::new(1), state_id, Rc::clone(&component));

    let first = [Dyn::num(1.0), Dyn::flag(false), Dyn::text("first")];
    __brul_attach_counter_apply(&mut ctx, state_id, &first).expect("first apply");
    let second = [Dyn::num(2.5), Dyn::flag(true), Dyn::text("second")];
    __brul_attach_counter_apply(&mut ctx, state_id, &second).expect("second apply");

    let cell = ctx.get_state::<CounterState>().expect("state created");
    assert_eq!(
        *cell.borrow(),
        CounterState {
            amount: 2.5,
            flag: true,
            label: "second".to_owned(),
        }
    );
    drop(cell);

    let log = LOG.with(|log| log.borrow().clone());
    assert_eq!(
        log.iter()
            .filter(|entry| entry.as_str() == "create")
            .count(),
        1
    );
    assert_eq!(log[1], "1|false|first");
    assert_eq!(log[2], "2.5|true|second");

    let key = brul::gui::KeyPress::from("Enter");
    let modifiers = brul::gui::KeyModifiers::default();
    let key_event = UiEvent::widget(brul::gui::WidgetId::new(1), Event::key(key, modifiers));
    assert!(__brul_attach_counter_event(&mut ctx, state_id, &key_event));
    assert!(!__brul_attach_counter_event(
        &mut ctx,
        state_id,
        &UiEvent::window(Event::submit())
    ));

    __brul_attach_counter_cleanup(&mut ctx, state_id);
    assert!(ctx.get_state::<CounterState>().is_none());

    let log = LOG.with(|log| log.borrow().clone());
    assert_eq!(log.last().map(String::as_str), Some("cleanup:2.5"));
}
