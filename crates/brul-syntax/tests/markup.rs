use brul_syntax::ast::{Expr, UnaryOp};
use brul_syntax::markup::{
    Arg, Block, BlockItem, Component, Declaration, Element, Event, EventParam, File, Member, Prop,
    PropType, Window, parse_file,
};

#[test]
fn component_with_empty_block_parses() {
    let file = parse_file("component CheckRow { }").unwrap();
    assert_eq!(
        file.declarations,
        vec![Declaration::Component(Component {
            name: "CheckRow".to_owned(),
            members: Vec::new(),
        })]
    );
}

#[test]
fn args_parse_classes_names_and_handlers() {
    let source = r#"window Picker (root, title: "Picker", width: 800) { }"#;
    let file = parse_file(source).unwrap();
    assert_eq!(
        file.declarations,
        vec![Declaration::Window(Window {
            name: "Picker".to_owned(),
            args: vec![
                Arg::Class("root".to_owned()),
                Arg::Named {
                    name: "title".to_owned(),
                    value: Expr::String("Picker".to_owned()),
                },
                Arg::Named {
                    name: "width".to_owned(),
                    value: Expr::Number(800.0),
                },
            ],
            members: Vec::new(),
        })]
    );
}

#[test]
fn element_args_parse_handlers_and_expressions() {
    let source = r#"component CheckRow {
        Button (check-row, disabled: !done, on click: toggle) { }
    }"#;
    let file = parse_file(source).unwrap();
    let Declaration::Component(component) = &file.declarations[0] else {
        panic!("expected a component");
    };
    assert_eq!(
        component.members,
        vec![Member::Element(Element {
            name: "Button".to_owned(),
            args: vec![
                Arg::Class("check-row".to_owned()),
                Arg::Named {
                    name: "disabled".to_owned(),
                    value: Expr::Unary {
                        op: UnaryOp::Not,
                        operand: Box::new(Expr::Ident("done".to_owned())),
                    },
                },
                Arg::Handler {
                    event: "click".to_owned(),
                    handler: "toggle".to_owned(),
                },
            ],
            block: Block { items: Vec::new() },
        })]
    );
}

#[test]
fn arg_errors_point_at_the_source() {
    let error = parse_file("component A { Button (on: x) { } }").unwrap_err();
    assert_eq!(error.message, "expected name");
    assert_eq!((error.line, error.column), (1, 25));

    let error = parse_file("component A { Button (root title) { } }").unwrap_err();
    assert_eq!(error.message, "expected ',' or ')'");
    assert_eq!((error.line, error.column), (1, 28));
}

#[test]
fn props_with_types_and_defaults_parse() {
    let source = r#"component CheckRow {
        prop label: String = ""
        prop checked: Bool = false
        prop count: Number = 0
        prop icon: Image = null
        prop plain: Bool
    }"#;
    let file = parse_file(source).unwrap();
    let Declaration::Component(component) = &file.declarations[0] else {
        panic!("expected a component");
    };
    assert_eq!(
        component.members,
        vec![
            Member::Prop(Prop {
                name: "label".to_owned(),
                ty: PropType::String,
                default: Some(Expr::String(String::new())),
            }),
            Member::Prop(Prop {
                name: "checked".to_owned(),
                ty: PropType::Bool,
                default: Some(Expr::Bool(false)),
            }),
            Member::Prop(Prop {
                name: "count".to_owned(),
                ty: PropType::Number,
                default: Some(Expr::Number(0.0)),
            }),
            Member::Prop(Prop {
                name: "icon".to_owned(),
                ty: PropType::Image,
                default: Some(Expr::Null),
            }),
            Member::Prop(Prop {
                name: "plain".to_owned(),
                ty: PropType::Bool,
                default: None,
            }),
        ]
    );
}

#[test]
fn events_parse_params() {
    let source = "component A { event clicked(i: Number) event toggled() event moved(x: Number, y: Number) }";
    let file = parse_file(source).unwrap();
    let Declaration::Component(component) = &file.declarations[0] else {
        panic!("expected a component");
    };
    assert_eq!(
        component.members,
        vec![
            Member::Event(Event {
                name: "clicked".to_owned(),
                params: vec![EventParam {
                    name: "i".to_owned(),
                    ty: PropType::Number,
                }],
            }),
            Member::Event(Event {
                name: "toggled".to_owned(),
                params: Vec::new(),
            }),
            Member::Event(Event {
                name: "moved".to_owned(),
                params: vec![
                    EventParam {
                        name: "x".to_owned(),
                        ty: PropType::Number,
                    },
                    EventParam {
                        name: "y".to_owned(),
                        ty: PropType::Number,
                    },
                ],
            }),
        ]
    );
}

#[test]
fn declarations_and_comments_mix() {
    let source = "// header\ncomponent A { }\n\nwindow B (root) { } // tail\n";
    assert_eq!(parse_file(source).unwrap().declarations.len(), 2);
}

#[test]
fn empty_input_is_an_empty_file() {
    assert_eq!(
        parse_file("  // only a comment\n").unwrap(),
        File {
            declarations: Vec::new(),
        }
    );
}

#[test]
fn declaration_errors_point_at_the_source() {
    let error = parse_file("view X { }").unwrap_err();
    assert_eq!(error.message, "expected 'component' or 'window'");
    assert_eq!((error.line, error.column), (1, 1));

    let error = parse_file("component { }").unwrap_err();
    assert_eq!(error.message, "expected name");
    assert_eq!((error.line, error.column), (1, 11));

    let error = parse_file("component A").unwrap_err();
    assert_eq!(error.message, "expected '{'");
    assert_eq!((error.line, error.column), (1, 12));

    let error = parse_file("component A { x }").unwrap_err();
    assert_eq!(error.message, "expected '{'");
    assert_eq!((error.line, error.column), (1, 17));

    let error = parse_file("window Picker () { }").unwrap_err();
    assert_eq!(error.message, "expected name");
    assert_eq!((error.line, error.column), (1, 16));
}

#[test]
fn nested_elements_parse() {
    let source = "component CheckRow { Row { Text { } View { View { } } } }";
    let file = parse_file(source).unwrap();
    let Declaration::Component(component) = &file.declarations[0] else {
        panic!("expected a component");
    };
    let view = || Element {
        name: "View".to_owned(),
        args: Vec::new(),
        block: Block { items: Vec::new() },
    };
    assert_eq!(
        component.members,
        vec![Member::Element(Element {
            name: "Row".to_owned(),
            args: Vec::new(),
            block: Block {
                items: vec![
                    BlockItem::Element(Element {
                        name: "Text".to_owned(),
                        args: Vec::new(),
                        block: Block { items: Vec::new() },
                    }),
                    BlockItem::Element(Element {
                        name: "View".to_owned(),
                        args: Vec::new(),
                        block: Block {
                            items: vec![BlockItem::Element(view())],
                        },
                    }),
                ],
            },
        })]
    );
}

#[test]
fn element_errors_point_at_the_source() {
    let error = parse_file("component A { View }").unwrap_err();
    assert_eq!(error.message, "expected '{'");
    assert_eq!((error.line, error.column), (1, 20));

    let error = parse_file("component A { View {").unwrap_err();
    assert_eq!(error.message, "expected '}'");
    assert_eq!((error.line, error.column), (1, 21));
}

#[test]
fn member_errors_point_at_the_source() {
    let error = parse_file("component A { prop x: Text = 0 }").unwrap_err();
    assert_eq!(error.message, "expected property type");
    assert_eq!((error.line, error.column), (1, 23));

    let error = parse_file("component A { prop x Bool }").unwrap_err();
    assert_eq!(error.message, "expected ':'");
    assert_eq!((error.line, error.column), (1, 22));

    let error = parse_file("component A { event clicked(i) }").unwrap_err();
    assert_eq!(error.message, "expected ':'");
    assert_eq!((error.line, error.column), (1, 30));

    let error = parse_file("component A { prop x: Bool = }").unwrap_err();
    assert_eq!(error.message, "expected expression");
    assert_eq!((error.line, error.column), (1, 30));
}
