#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ElementKind {
    Leaf,
    Container,
}

#[derive(Clone, Copy, Debug)]
pub struct PropSpec {
    pub name: &'static str,
    pub ty: &'static str,
    pub target: &'static str,
}

#[derive(Clone, Copy, Debug)]
pub struct EventSpec {
    pub name: &'static str,
    pub params: &'static [&'static str],
}

#[derive(Clone, Copy, Debug)]
pub struct ElementSpec {
    pub name: &'static str,
    pub kind: ElementKind,
    pub props: &'static [PropSpec],
    pub events: &'static [EventSpec],
}

const FLAGS: &[PropSpec] = &[
    PropSpec {
        name: "focusable",
        ty: "Bool",
        target: "Flag",
    },
    PropSpec {
        name: "multiline",
        ty: "Bool",
        target: "Flag",
    },
    PropSpec {
        name: "modal",
        ty: "Bool",
        target: "Flag",
    },
];

const PROPS: &[PropSpec] = &[
    PropSpec {
        name: "checked",
        ty: "Bool",
        target: "Checked",
    },
    PropSpec {
        name: "selected",
        ty: "Bool",
        target: "Selected",
    },
    PropSpec {
        name: "disabled",
        ty: "Bool",
        target: "Disabled",
    },
    PropSpec {
        name: "visible",
        ty: "Bool",
        target: "Visible",
    },
    PropSpec {
        name: "class",
        ty: "String",
        target: "Class",
    },
    PropSpec {
        name: "x",
        ty: "Number",
        target: "Offset",
    },
    PropSpec {
        name: "y",
        ty: "Number",
        target: "Offset",
    },
    PropSpec {
        name: "focusable",
        ty: "Bool",
        target: "Flag",
    },
    PropSpec {
        name: "multiline",
        ty: "Bool",
        target: "Flag",
    },
    PropSpec {
        name: "modal",
        ty: "Bool",
        target: "Flag",
    },
];

const EVENTS: &[EventSpec] = &[
    EventSpec {
        name: "click",
        params: &[],
    },
    EventSpec {
        name: "press",
        params: &[],
    },
    EventSpec {
        name: "release",
        params: &[],
    },
    EventSpec {
        name: "enter",
        params: &[],
    },
    EventSpec {
        name: "leave",
        params: &[],
    },
    EventSpec {
        name: "drag",
        params: &[],
    },
    EventSpec {
        name: "focus",
        params: &[],
    },
    EventSpec {
        name: "blur",
        params: &[],
    },
    EventSpec {
        name: "key",
        params: &["Object"],
    },
    EventSpec {
        name: "input",
        params: &["Object"],
    },
    EventSpec {
        name: "submit",
        params: &[],
    },
    EventSpec {
        name: "dismiss",
        params: &[],
    },
];

const VIDEO_PROPS: &[PropSpec] = &[
    PropSpec {
        name: "source",
        ty: "String",
        target: "Text",
    },
    PropSpec {
        name: "playing",
        ty: "Bool",
        target: "Flag",
    },
];

const VIDEO_EVENTS: &[EventSpec] = &[
    EventSpec {
        name: "ended",
        params: &[],
    },
    EventSpec {
        name: "error",
        params: &["String"],
    },
];

const WEBVIEW_PROPS: &[PropSpec] = &[
    PropSpec {
        name: "source",
        ty: "String",
        target: "Text",
    },
    PropSpec {
        name: "visible",
        ty: "Bool",
        target: "Flag",
    },
];

const WEBVIEW_EVENTS: &[EventSpec] = &[EventSpec {
    name: "message",
    params: &["String"],
}];

pub const ELEMENTS: &[ElementSpec] = &[
    ElementSpec {
        name: "Container",
        kind: ElementKind::Container,
        props: PROPS,
        events: EVENTS,
    },
    ElementSpec {
        name: "Text",
        kind: ElementKind::Leaf,
        props: PROPS,
        events: EVENTS,
    },
    ElementSpec {
        name: "TextInput",
        kind: ElementKind::Leaf,
        props: PROPS,
        events: EVENTS,
    },
    ElementSpec {
        name: "Image",
        kind: ElementKind::Leaf,
        props: PROPS,
        events: EVENTS,
    },
    ElementSpec {
        name: "Layer",
        kind: ElementKind::Container,
        props: PROPS,
        events: EVENTS,
    },
    ElementSpec {
        name: "Video",
        kind: ElementKind::Leaf,
        props: VIDEO_PROPS,
        events: VIDEO_EVENTS,
    },
    ElementSpec {
        name: "WebView",
        kind: ElementKind::Leaf,
        props: WEBVIEW_PROPS,
        events: WEBVIEW_EVENTS,
    },
];

pub fn find(name: &str) -> Option<&'static ElementSpec> {
    ELEMENTS.iter().find(|element| element.name == name)
}

pub fn prop(element: &ElementSpec, name: &str) -> Option<&'static PropSpec> {
    element.props.iter().find(|prop| prop.name == name)
}

pub fn event(element: &ElementSpec, name: &str) -> Option<&'static EventSpec> {
    element.events.iter().find(|event| event.name == name)
}

pub fn flags() -> &'static [PropSpec] {
    FLAGS
}
