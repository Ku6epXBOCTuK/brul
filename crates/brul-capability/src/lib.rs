use std::collections::BTreeSet;
use std::fmt;

use brul_case::snake;
use brul_event::{EventKind, RawKind};
use proc_macro2::TokenStream;
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::{Attribute, Ident, ItemFn, LitStr, Token};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WidgetAttr {
    pub name: Option<String>,
    pub events: Vec<EventArg>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventArg {
    pub name: String,
    pub params: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttachEvents {
    pub semantic: BTreeSet<EventKind>,
    pub raw: BTreeSet<RawKind>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttachAttr {
    pub name: Option<String>,
    pub events: AttachEvents,
    pub update: Option<Ident>,
    pub event: Option<Ident>,
    pub raw_event: Option<Ident>,
    pub cleanup: Option<Ident>,
}

fn attribute<'a>(function: &'a ItemFn, name: &str) -> Option<&'a Attribute> {
    function.attrs.iter().find(|attribute| {
        attribute
            .path()
            .segments
            .last()
            .is_some_and(|segment| segment.ident == name)
    })
}

pub fn widget_attribute(function: &ItemFn) -> Option<&Attribute> {
    attribute(function, "widget")
}

pub fn attach_attribute(function: &ItemFn) -> Option<&Attribute> {
    attribute(function, "attach")
}

pub fn is_widget(function: &ItemFn) -> bool {
    widget_attribute(function).is_some()
}

pub fn is_attach(function: &ItemFn) -> bool {
    attach_attribute(function).is_some()
}

fn attribute_tokens(attribute: &Attribute) -> TokenStream {
    match &attribute.meta {
        syn::Meta::List(list) => list.tokens.clone(),
        _ => TokenStream::new(),
    }
}

pub fn parse_widget(tokens: TokenStream) -> syn::Result<WidgetAttr> {
    let arguments = Punctuated::<WidgetArgument, Token![,]>::parse_terminated.parse2(tokens)?;
    let mut result = WidgetAttr::default();
    for argument in arguments {
        match argument {
            WidgetArgument::Name(name) => result.name = Some(name.value()),
            WidgetArgument::Events(events) => result.events = events,
        }
    }
    Ok(result)
}

pub fn widget_attr(attribute: &Attribute) -> syn::Result<WidgetAttr> {
    parse_widget(attribute_tokens(attribute))
}

pub fn parse_attach(tokens: TokenStream) -> syn::Result<AttachAttr> {
    let arguments = Punctuated::<AttachArgument, Token![,]>::parse_terminated.parse2(tokens)?;
    let mut result = AttachAttr::default();
    for argument in arguments {
        match argument {
            AttachArgument::Name(name) => result.name = Some(name.value()),
            AttachArgument::Events(events) => result.events.semantic.extend(events.semantic),
            AttachArgument::RawEvents(events) => result.events.raw.extend(events.raw),
            AttachArgument::Update(hook) => result.update = Some(hook),
            AttachArgument::Event(hook) => result.event = Some(hook),
            AttachArgument::RawEvent(hook) => result.raw_event = Some(hook),
            AttachArgument::Cleanup(hook) => result.cleanup = Some(hook),
        }
    }
    Ok(result)
}

pub fn attach_attr(attribute: &Attribute) -> syn::Result<AttachAttr> {
    parse_attach(attribute_tokens(attribute))
}

fn value<T: Parse>(input: ParseStream) -> syn::Result<T> {
    input.parse::<Token![=]>()?;
    input.parse()
}

enum WidgetArgument {
    Name(LitStr),
    Events(Vec<EventArg>),
}

impl Parse for WidgetArgument {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        match key.to_string().as_str() {
            "name" => Ok(Self::Name(value(input)?)),
            "events" => {
                let content;
                syn::parenthesized!(content in input);
                let events = Punctuated::<EventArgument, Token![,]>::parse_terminated(&content)?;
                Ok(Self::Events(
                    events.into_iter().map(EventArg::from).collect(),
                ))
            }
            _ => Err(syn::Error::new(key.span(), "expected `name` or `events`")),
        }
    }
}

struct EventArgument {
    name: String,
    params: Vec<String>,
}

impl Parse for EventArgument {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;
        let content;
        syn::bracketed!(content in input);
        let params = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
        Ok(Self {
            name: name.to_string(),
            params: params.into_iter().map(|param| param.to_string()).collect(),
        })
    }
}

impl From<EventArgument> for EventArg {
    fn from(argument: EventArgument) -> Self {
        Self {
            name: argument.name,
            params: argument.params,
        }
    }
}

enum AttachArgument {
    Name(LitStr),
    Events(AttachEvents),
    RawEvents(AttachEvents),
    Update(Ident),
    Event(Ident),
    RawEvent(Ident),
    Cleanup(Ident),
}

#[derive(Clone, Copy)]
enum EventFamily {
    Semantic,
    Raw,
}

impl EventFamily {
    fn keyword(self) -> &'static str {
        match self {
            Self::Semantic => "events",
            Self::Raw => "raw_events",
        }
    }

    fn other(self) -> &'static str {
        match self {
            Self::Semantic => "raw_events",
            Self::Raw => "events",
        }
    }
}

fn parse_events(input: ParseStream, family: EventFamily) -> syn::Result<AttachEvents> {
    let content;
    syn::parenthesized!(content in input);
    let events = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
    let mut result = AttachEvents::default();
    for event in events {
        let name = event.to_string();
        match family {
            EventFamily::Semantic => match name.parse::<EventKind>() {
                Ok(kind) => {
                    result.semantic.insert(kind);
                }
                Err(_) if name.parse::<RawKind>().is_ok() => {
                    return Err(syn::Error::new(
                        event.span(),
                        WrongAttachEventFamily {
                            name,
                            expected: family.keyword(),
                            found: family.other(),
                        },
                    ));
                }
                Err(error) => {
                    return Err(syn::Error::new(event.span(), UnknownAttachEvent(error.0)));
                }
            },
            EventFamily::Raw => match name.parse::<RawKind>() {
                Ok(kind) => {
                    result.raw.insert(kind);
                }
                Err(_) if name.parse::<EventKind>().is_ok() => {
                    return Err(syn::Error::new(
                        event.span(),
                        WrongAttachEventFamily {
                            name,
                            expected: family.keyword(),
                            found: family.other(),
                        },
                    ));
                }
                Err(error) => {
                    return Err(syn::Error::new(event.span(), UnknownAttachEvent(error.0)));
                }
            },
        }
    }
    Ok(result)
}

impl Parse for AttachArgument {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        match key.to_string().as_str() {
            "name" => Ok(Self::Name(value(input)?)),
            "events" => Ok(Self::Events(parse_events(input, EventFamily::Semantic)?)),
            "raw_events" => Ok(Self::RawEvents(parse_events(input, EventFamily::Raw)?)),
            "update" => Ok(Self::Update(value(input)?)),
            "event" => Ok(Self::Event(value(input)?)),
            "raw_event" => Ok(Self::RawEvent(value(input)?)),
            "cleanup" => Ok(Self::Cleanup(value(input)?)),
            _ => Err(syn::Error::new(key.span(), "unknown attachment argument")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnknownAttachEvent(pub String);

impl fmt::Display for UnknownAttachEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown attachment event '{}'", self.0)
    }
}

impl std::error::Error for UnknownAttachEvent {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WrongAttachEventFamily {
    pub name: String,
    pub expected: &'static str,
    pub found: &'static str,
}

impl fmt::Display for WrongAttachEventFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "attachment event '{}' belongs to {}, use {}()",
            self.name, self.found, self.expected
        )
    }
}

impl std::error::Error for WrongAttachEventFamily {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachShims {
    pub apply: Ident,
    pub event: Ident,
    pub raw_event: Ident,
    pub cleanup: Ident,
}

fn shim(prefix: &str, function: &Ident, suffix: &str) -> Ident {
    Ident::new(
        &format!("{prefix}_{}{suffix}", snake(&function.to_string())),
        function.span(),
    )
}

pub fn widget_shim(function: &Ident) -> Ident {
    shim("__brul_widget", function, "")
}

pub fn attach_shims(function: &Ident) -> AttachShims {
    AttachShims {
        apply: shim("__brul_attach", function, "_apply"),
        event: shim("__brul_attach", function, "_event"),
        raw_event: shim("__brul_attach", function, "_raw_event"),
        cleanup: shim("__brul_attach", function, "_cleanup"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(source: &str) -> TokenStream {
        source.parse().expect("tokens")
    }

    #[test]
    fn parses_full_attach_syntax() {
        let attribute = parse_attach(tokens(
            "name = \"counter\", events(Key, Input, Submit, Focus, Blur, Click), \
             raw_events(Ime), update = counter_update, event = counter_event, \
             raw_event = counter_raw_event, cleanup = counter_cleanup",
        ))
        .expect("attach attribute");
        assert_eq!(attribute.name.as_deref(), Some("counter"));
        assert_eq!(
            attribute.events,
            AttachEvents {
                semantic: [
                    EventKind::Key,
                    EventKind::Input,
                    EventKind::Submit,
                    EventKind::Focus,
                    EventKind::Blur,
                    EventKind::Click,
                ]
                .into_iter()
                .collect(),
                raw: [RawKind::Ime].into_iter().collect(),
            }
        );
        assert_eq!(
            attribute.update.as_ref().unwrap().to_string(),
            "counter_update"
        );
        assert_eq!(
            attribute.event.as_ref().unwrap().to_string(),
            "counter_event"
        );
        assert_eq!(
            attribute.raw_event.as_ref().unwrap().to_string(),
            "counter_raw_event"
        );
        assert_eq!(
            attribute.cleanup.as_ref().unwrap().to_string(),
            "counter_cleanup"
        );
    }

    #[test]
    fn parses_widget_events_with_parameters() {
        let attribute = parse_widget(tokens(
            "name = \"Gauge\", events(changed: [Number, String], reset: [])",
        ))
        .expect("widget attribute");
        assert_eq!(attribute.name.as_deref(), Some("Gauge"));
        assert_eq!(
            attribute.events,
            vec![
                EventArg {
                    name: "changed".to_owned(),
                    params: vec!["Number".to_owned(), "String".to_owned()],
                },
                EventArg {
                    name: "reset".to_owned(),
                    params: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn reads_attributes_from_a_function() {
        let function = syn::parse_str::<ItemFn>("#[brul::widget(name = \"Gauge\")]\nfn gauge() {}")
            .expect("function");
        assert!(is_widget(&function));
        assert!(!is_attach(&function));
        let attribute = widget_attribute(&function).expect("widget attribute");
        assert_eq!(
            widget_attr(attribute).unwrap().name.as_deref(),
            Some("Gauge")
        );
    }

    #[test]
    fn event_kinds_round_trip_names_and_fields() {
        for kind in EventKind::ALL {
            assert_eq!(kind.to_string().parse::<EventKind>().unwrap(), *kind);
            assert_eq!(kind.field().parse::<EventKind>().unwrap(), *kind);
        }
    }

    #[test]
    fn rejects_unknown_events() {
        let error =
            parse_attach(tokens("name = \"editor\", events(Pointer)")).expect_err("unknown event");
        assert_eq!(error.to_string(), "unknown attachment event 'Pointer'");
    }

    #[test]
    fn raw_events_can_name_a_key_shared_by_both_families() {
        let attribute =
            parse_attach(tokens("name = \"keys\", raw_events(Key, KeyNoop)")).expect("raw key");
        assert!(attribute.events.semantic.is_empty());
        assert_eq!(
            attribute.events.raw,
            [RawKind::Key, RawKind::KeyNoop].into_iter().collect()
        );
    }

    #[test]
    fn rejects_an_event_from_the_other_family() {
        let semantic =
            parse_attach(tokens("name = \"editor\", events(Ime)")).expect_err("raw name in events");
        assert_eq!(
            semantic.to_string(),
            "attachment event 'Ime' belongs to raw_events, use events()"
        );
        let raw = parse_attach(tokens("name = \"editor\", raw_events(Click)"))
            .expect_err("semantic name in raw_events");
        assert_eq!(
            raw.to_string(),
            "attachment event 'Click' belongs to events, use raw_events()"
        );
    }

    #[test]
    fn repeated_event_arguments_are_merged() {
        let attribute = parse_attach(tokens(
            "name = \"editor\", events(Key), raw_events(Ime), raw_events(Press)",
        ))
        .expect("merged events");
        assert_eq!(
            attribute.events.semantic,
            [EventKind::Key].into_iter().collect()
        );
        assert_eq!(
            attribute.events.raw,
            [RawKind::Ime, RawKind::Press].into_iter().collect()
        );
    }

    #[test]
    fn derives_shim_names_from_the_function() {
        let function = syn::parse_str::<ItemFn>("fn text_input_box() {}").expect("function");
        let shims = attach_shims(&function.sig.ident);
        assert_eq!(
            widget_shim(&function.sig.ident).to_string(),
            "__brul_widget_text_input_box"
        );
        assert_eq!(
            shims.apply.to_string(),
            "__brul_attach_text_input_box_apply"
        );
        assert_eq!(
            shims.event.to_string(),
            "__brul_attach_text_input_box_event"
        );
        assert_eq!(
            shims.raw_event.to_string(),
            "__brul_attach_text_input_box_raw_event"
        );
        assert_eq!(
            shims.cleanup.to_string(),
            "__brul_attach_text_input_box_cleanup"
        );
    }
}
