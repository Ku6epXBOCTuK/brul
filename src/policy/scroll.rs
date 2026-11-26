use crate::{CapabilityContext, Dispatch, RawEvent, RawUiEvent, ViewError};

pub struct ScrollPolicy;

#[brul::attach(name = "scroll", raw_events(Wheel), raw_event = scroll_raw)]
pub fn scroll(_build: &CapabilityContext) -> Result<ScrollPolicy, ViewError> {
    Ok(ScrollPolicy)
}

fn scroll_raw(
    _state: &mut ScrollPolicy,
    build: &CapabilityContext,
    event: &RawUiEvent,
) -> Dispatch {
    if build.handled() {
        return Dispatch::CONTINUE;
    }
    let RawEvent::Wheel { delta } = event.event else {
        return Dispatch::CONTINUE;
    };
    if delta == 0 {
        return Dispatch::CONTINUE;
    }
    let Some(id) = event.target.widget() else {
        return Dispatch::CONTINUE;
    };
    let Some(target) = build.nearest_scrollable(id) else {
        return Dispatch::CONTINUE;
    };
    if build
        .element_for(target)
        .is_some_and(|element| element.scroll_by(delta))
    {
        Dispatch::handled()
    } else {
        Dispatch::CONTINUE
    }
}
