use std::cell::RefCell;
use std::rc::Rc;

use brul::{
    CapabilityContext, Component, ComponentHandle, Dispatch, Dyn, Event, RawUiEvent, Services,
    SharedStates, StateId, States, UiEvent, ViewError,
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
    raw_event = counter_raw_event,
    cleanup = counter_cleanup
)]
fn counter(
    _ctx: &CapabilityContext,
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
    _ctx: &CapabilityContext,
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

fn counter_event(_state: &mut CounterState, _ctx: &CapabilityContext, event: &UiEvent) -> bool {
    matches!(event.event, Event::Key { .. })
}

fn counter_raw_event(
    _state: &mut CounterState,
    _ctx: &CapabilityContext,
    event: &RawUiEvent,
) -> bool {
    matches!(event.event, brul::gui::RawEvent::Key { .. })
}

fn counter_cleanup(state: &mut CounterState, _ctx: &CapabilityContext) {
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
    let state_id = StateId::source("test", "forty_two");
    let ctx = CapabilityContext::new(
        Services {
            states: Rc::clone(&states),
            ..Services::default()
        },
        tree,
        brul::EventTarget::Widget(brul::gui::WidgetId::new(1)),
        state_id,
        Rc::clone(&component),
        None,
    );

    let first = [Dyn::num(1.0), Dyn::flag(false), Dyn::text("first")];
    __brul_attach_counter_apply(&ctx, &first).expect("first apply");
    let second = [Dyn::num(2.5), Dyn::flag(true), Dyn::text("second")];
    __brul_attach_counter_apply(&ctx, &second).expect("second apply");

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
    assert_eq!(
        __brul_attach_counter_event(&ctx, &key_event),
        Dispatch::stop()
    );
    assert_eq!(
        __brul_attach_counter_event(&ctx, &UiEvent::window(Event::submit())),
        Dispatch::CONTINUE
    );

    let raw_key = RawUiEvent::widget(
        brul::gui::WidgetId::new(1),
        brul::gui::RawEvent::key(
            brul::gui::KeyPress::from("Enter"),
            brul::gui::KeyModifiers::default(),
        ),
    );
    assert_eq!(
        __brul_attach_counter_raw_event(&ctx, &raw_key),
        Dispatch::stop()
    );

    __brul_attach_counter_cleanup(&ctx);
    assert!(ctx.get_state::<CounterState>().is_none());

    let log = LOG.with(|log| log.borrow().clone());
    assert_eq!(log.last().map(String::as_str), Some("cleanup:2.5"));
}

fn counter_context() -> CapabilityContext {
    let component: ComponentHandle = Rc::new(RefCell::new(Component::new()));
    let tree = Rc::new(RefCell::new(brul::gui::WidgetTree::new(
        brul::gui::Widget::text("root"),
    )));
    CapabilityContext::new(
        Services {
            states: Rc::new(RefCell::new(States::default())),
            ..Services::default()
        },
        tree,
        brul::EventTarget::Widget(brul::gui::WidgetId::new(1)),
        StateId::source("test", "reentrant"),
        component,
        None,
    )
}

fn seed_counter(ctx: &CapabilityContext) {
    let params = [Dyn::num(1.0), Dyn::flag(false), Dyn::text("first")];
    __brul_attach_counter_apply(ctx, &params).expect("apply");
}

#[test]
fn apply_panics_when_state_is_already_borrowed() {
    let ctx = counter_context();
    seed_counter(&ctx);
    let cell = ctx.get_state::<CounterState>().expect("state");
    let guard = cell.borrow_mut();

    let params = [Dyn::num(2.0), Dyn::flag(true), Dyn::text("second")];
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        __brul_attach_counter_apply(&ctx, &params)
    }));
    drop(guard);

    assert!(result.is_err());
    assert_eq!(cell.borrow().amount, 1.0);
}

#[test]
#[should_panic(expected = "already borrowed")]
fn event_panics_when_state_is_already_borrowed() {
    let ctx = counter_context();
    seed_counter(&ctx);
    let cell = ctx.get_state::<CounterState>().expect("state");
    let _guard = cell.borrow_mut();
    let key = UiEvent::widget(
        brul::gui::WidgetId::new(1),
        Event::key(
            brul::gui::KeyPress::from("Enter"),
            brul::gui::KeyModifiers::default(),
        ),
    );

    let _ = __brul_attach_counter_event(&ctx, &key);
}

#[test]
#[should_panic(expected = "already borrowed")]
fn raw_event_panics_when_state_is_already_borrowed() {
    let ctx = counter_context();
    seed_counter(&ctx);
    let cell = ctx.get_state::<CounterState>().expect("state");
    let _guard = cell.borrow_mut();
    let raw = RawUiEvent::widget(
        brul::gui::WidgetId::new(1),
        brul::gui::RawEvent::key(
            brul::gui::KeyPress::from("Enter"),
            brul::gui::KeyModifiers::default(),
        ),
    );

    let _ = __brul_attach_counter_raw_event(&ctx, &raw);
}

#[test]
fn cleanup_panics_and_keeps_state_when_state_is_already_borrowed() {
    let ctx = counter_context();
    seed_counter(&ctx);
    let cell = ctx.get_state::<CounterState>().expect("state");
    let guard = cell.borrow_mut();

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        __brul_attach_counter_cleanup(&ctx);
    }));
    drop(guard);

    assert!(result.is_err());
    assert!(ctx.get_state::<CounterState>().is_some());
    assert_eq!(cell.borrow().amount, 1.0);
}
