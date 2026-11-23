use brul_syntax::ast::{BinaryOp, Expr, ExprKind, Position, TemplatePart, UnaryOp};
use brul_syntax::markup::{
    AnchorName, Arg, Block, BlockItem, Branch, Declaration, DeclarationKind, EachBlock, Element,
    Event, EventParam, File, IfBlock, Member, Prop, PropType, Widget, parse_file,
};

fn expr(kind: ExprKind) -> Expr {
    Expr::new(kind, Position::default())
}

fn ident(name: &str) -> Expr {
    expr(ExprKind::Ident(name.to_owned()))
}

fn string(value: &str) -> Expr {
    expr(ExprKind::String(value.to_owned()))
}

fn number(value: f64) -> Expr {
    expr(ExprKind::Number(value))
}

#[test]
fn component_with_empty_block_parses() {
    let file = parse_file("component CheckRow { }").unwrap();
    assert_eq!(
        file.declaration,
        Some(Declaration {
            name: "CheckRow".to_owned(),
            members: Vec::new(),
            position: Position::default(),
            kind: DeclarationKind::Component,
        })
    );
}

#[test]
fn args_parse_classes_names_and_handlers() {
    let source = r#"window Picker (root, title: "Picker", width: 800) { }"#;
    let file = parse_file(source).unwrap();
    assert_eq!(
        file.declaration,
        Some(Declaration {
            name: "Picker".to_owned(),
            members: Vec::new(),
            position: Position::default(),
            kind: DeclarationKind::Window {
                args: vec![
                    Arg::Class("root".to_owned()),
                    Arg::Named {
                        name: "title".to_owned(),
                        value: string("Picker"),
                    },
                    Arg::Named {
                        name: "width".to_owned(),
                        value: number(800.0),
                    },
                ],
            },
        })
    );
}

#[test]
fn element_args_parse_handlers_and_expressions() {
    let source = r#"component CheckRow {
        Button (check-row, disabled: !done, on click: toggle) { }
    }"#;
    let file = parse_file(source).unwrap();
    let component = file.declaration.as_ref().unwrap();
    assert_eq!(
        component.members,
        vec![Member::Element(Element {
            position: Position::default(),
            name: "Button".to_owned(),
            widget: None,
            args: vec![
                Arg::Class("check-row".to_owned()),
                Arg::Named {
                    name: "disabled".to_owned(),
                    value: expr(ExprKind::Unary {
                        op: UnaryOp::Not,
                        operand: Box::new(ident("done")),
                    }),
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
fn element_args_parse_attachments_with_params() {
    let source = r#"component Editor {
        TextInput (field, use: editor(value: draft, multiline: true)) { draft }
    }"#;
    let file = parse_file(source).unwrap();
    let Member::Element(element) = &file.declaration.as_ref().unwrap().members[0] else {
        panic!("element");
    };
    let Arg::Attach { name, params, .. } = &element.args[1] else {
        panic!("attach arg");
    };
    assert_eq!(name, "editor");
    assert_eq!(params.len(), 2);
    assert_eq!(params[0].0, "value");
    assert_eq!(params[0].1.kind, ExprKind::Ident("draft".to_owned()));
    assert_eq!(params[1].0, "multiline");
    assert_eq!(params[1].1.kind, ExprKind::Bool(true));
}

#[test]
fn attachment_without_params_parses() {
    let file = parse_file("window Demo (root) { TextInput (use: editor) { draft } }").unwrap();
    let declaration = file.declaration.as_ref().unwrap();
    let Member::Element(element) = &declaration.members[0] else {
        panic!("element");
    };
    assert!(matches!(
        &element.args[0],
        Arg::Attach { name, params, .. } if name == "editor" && params.is_empty()
    ));
}

#[test]
fn a_malformed_attachment_is_rejected() {
    assert!(
        parse_file("window Demo (root) { TextInput (use: editor(a: 1 b: 2)) { draft } }").is_err()
    );
}

#[test]
fn anchor_name_parses_separately_from_classes() {
    let source = r#"component Tip {
        Container (trigger, #tip-target) {
            Container (bubble, anchor: @tip-target) { }
        }
    }"#;
    let file = parse_file(source).unwrap();
    let declaration = file.declaration.as_ref().unwrap();
    let Member::Element(trigger) = &declaration.members[0] else {
        panic!("expected an element")
    };
    assert_eq!(trigger.args[0], Arg::Class("trigger".to_owned()));
    assert!(matches!(
        &trigger.args[1],
        Arg::Name(AnchorName { name, .. }) if name == "tip-target"
    ));
    let BlockItem::Element(bubble) = &trigger.block.items[0] else {
        panic!("expected an element")
    };
    assert_eq!(bubble.args[0], Arg::Class("bubble".to_owned()));
    assert!(matches!(
        &bubble.args[1],
        Arg::Anchor(AnchorName { name, .. }) if name == "tip-target"
    ));
}

#[test]
fn arg_errors_point_at_the_source() {
    let error = parse_file("component A { Button (on: x) { } }").unwrap_err();
    assert_eq!(error.message(), "expected name");
    assert_eq!((error.line(), error.column()), (1, 25));

    let error = parse_file("component A { Button (root title) { } }").unwrap_err();
    assert_eq!(error.message(), "expected ',' or ')'");
    assert_eq!((error.line(), error.column()), (1, 28));
}

#[test]
fn props_with_types_and_defaults_parse() {
    let source = r#"component CheckRow {
        prop label: String = "";
        prop checked: Bool = false;
        prop count: Number = 0;
        prop icon: Image = null;
        prop plain: Bool;
    }"#;
    let file = parse_file(source).unwrap();
    let component = file.declaration.as_ref().unwrap();
    assert_eq!(
        component.members,
        vec![
            Member::Prop(Prop {
                name: "label".to_owned(),
                ty: PropType::String,
                default: Some(expr(ExprKind::String(String::new()))),
            }),
            Member::Prop(Prop {
                name: "checked".to_owned(),
                ty: PropType::Bool,
                default: Some(expr(ExprKind::Bool(false))),
            }),
            Member::Prop(Prop {
                name: "count".to_owned(),
                ty: PropType::Number,
                default: Some(number(0.0)),
            }),
            Member::Prop(Prop {
                name: "icon".to_owned(),
                ty: PropType::Image,
                default: Some(expr(ExprKind::Null)),
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
fn event_without_parentheses_takes_no_params() {
    let file = parse_file("component A { event clicked; }").unwrap();
    let component = file.declaration.as_ref().unwrap();
    assert_eq!(
        component.members,
        vec![Member::Event(Event {
            name: "clicked".to_owned(),
            params: Vec::new(),
        })]
    );
}

#[test]
fn events_parse_params() {
    let source = "component A { event clicked(i: Number); event toggled(); event moved(x: Number, y: Number); }";
    let file = parse_file(source).unwrap();
    let component = file.declaration.as_ref().unwrap();
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
fn a_second_declaration_is_rejected() {
    let source = "// header\ncomponent A { }\n\nwindow B (root) { } // tail\n";
    assert_eq!(
        parse_file(source).unwrap_err().message(),
        "a file can contain only one declaration"
    );
}

#[test]
fn empty_input_is_an_empty_file() {
    assert_eq!(
        parse_file("  // only a comment\n").unwrap(),
        File { declaration: None }
    );
}

#[test]
fn declaration_errors_point_at_the_source() {
    let error = parse_file("view X { }").unwrap_err();
    assert_eq!(error.message(), "expected 'component' or 'window'");
    assert_eq!((error.line(), error.column()), (1, 1));

    let error = parse_file("component { }").unwrap_err();
    assert_eq!(error.message(), "expected name");
    assert_eq!((error.line(), error.column()), (1, 11));

    let error = parse_file("component A").unwrap_err();
    assert_eq!(error.message(), "expected '{'");
    assert_eq!((error.line(), error.column()), (1, 12));

    let error = parse_file("component A { x }").unwrap_err();
    assert_eq!(error.message(), "expected '{'");
    assert_eq!((error.line(), error.column()), (1, 17));

    let error = parse_file("window Picker () { }").unwrap_err();
    assert_eq!(error.message(), "expected name");
    assert_eq!((error.line(), error.column()), (1, 16));
}

#[test]
fn nested_elements_parse() {
    let source = "component CheckRow { Container { Text { \"hi\" } Container { Container { } } } }";
    let file = parse_file(source).unwrap();
    let component = file.declaration.as_ref().unwrap();
    let view = || Element {
        position: Position::default(),
        name: "Container".to_owned(),
        widget: Some(Widget::Container),
        args: Vec::new(),
        block: Block { items: Vec::new() },
    };
    assert_eq!(
        component.members,
        vec![Member::Element(Element {
            position: Position::default(),
            name: "Container".to_owned(),
            widget: Some(Widget::Container),
            args: Vec::new(),
            block: Block {
                items: vec![
                    BlockItem::Element(Element {
                        position: Position::default(),
                        name: "Text".to_owned(),
                        widget: Some(Widget::Text),
                        args: Vec::new(),
                        block: Block {
                            items: vec![BlockItem::Expr(string("hi"))],
                        },
                    }),
                    BlockItem::Element(Element {
                        position: Position::default(),
                        name: "Container".to_owned(),
                        widget: Some(Widget::Container),
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
    let error = parse_file("component A { Container }").unwrap_err();
    assert_eq!(error.message(), "expected '{'");
    assert_eq!((error.line(), error.column()), (1, 25));

    let error = parse_file("component A { Container {").unwrap_err();
    assert_eq!(error.message(), "expected '}'");
    assert_eq!((error.line(), error.column()), (1, 26));
}

#[test]
fn member_errors_point_at_the_source() {
    let error = parse_file("component A { prop x: Text = 0; }").unwrap_err();
    assert_eq!(error.message(), "expected property type");
    assert_eq!((error.line(), error.column()), (1, 23));

    let error = parse_file("component A { prop x Bool; }").unwrap_err();
    assert_eq!(error.message(), "expected ':'");
    assert_eq!((error.line(), error.column()), (1, 22));

    let error = parse_file("component A { event clicked(i); }").unwrap_err();
    assert_eq!(error.message(), "expected ':'");
    assert_eq!((error.line(), error.column()), (1, 30));

    let error = parse_file("component A { event clicked(i: Number j: Number); }").unwrap_err();
    assert_eq!(error.message(), "expected ',' or ')'");

    let error = parse_file("component A { prop x: Bool = ; }").unwrap_err();
    assert_eq!(error.message(), "expected expression");
    assert_eq!((error.line(), error.column()), (1, 30));

    let error = parse_file("component A { prop x: Bool }").unwrap_err();
    assert_eq!(error.message(), "expected ';'");
    assert_eq!((error.line(), error.column()), (1, 28));

    let error = parse_file("component A { event clicked }").unwrap_err();
    assert_eq!(error.message(), "expected ';'");
    assert_eq!((error.line(), error.column()), (1, 29));
}

#[test]
fn if_else_blocks_parse() {
    let source = r#"component CheckRow {
        Container {
            @if (done) { Span { } } @else { Container { } }
            @if (failed) { Text { "no" } } @else if (retry) { Span { } }
        }
    }"#;
    let file = parse_file(source).unwrap();
    let component = file.declaration.as_ref().unwrap();
    let element = |name: &str| Element {
        position: Position::default(),
        name: name.to_owned(),
        widget: Widget::from_name(name),
        args: Vec::new(),
        block: Block { items: Vec::new() },
    };
    assert_eq!(
        component.members,
        vec![Member::Element(Element {
            position: Position::default(),
            name: "Container".to_owned(),
            widget: Some(Widget::Container),
            args: Vec::new(),
            block: Block {
                items: vec![
                    BlockItem::If(IfBlock {
                        branches: vec![Branch {
                            condition: ident("done"),
                            block: Block {
                                items: vec![BlockItem::Element(element("Span"))],
                            },
                            position: Position::default(),
                        }],
                        else_block: Some(Block {
                            items: vec![BlockItem::Element(element("Container"))],
                        }),
                        position: Position::default(),
                    }),
                    BlockItem::If(IfBlock {
                        branches: vec![
                            Branch {
                                condition: ident("failed"),
                                block: Block {
                                    items: vec![BlockItem::Element(Element {
                                        position: Position::default(),
                                        name: "Text".to_owned(),
                                        widget: Some(Widget::Text),
                                        args: Vec::new(),
                                        block: Block {
                                            items: vec![BlockItem::Expr(string("no"))],
                                        },
                                    })],
                                },
                                position: Position::default(),
                            },
                            Branch {
                                condition: ident("retry"),
                                block: Block {
                                    items: vec![BlockItem::Element(element("Span"))],
                                },
                                position: Position::default(),
                            },
                        ],
                        else_block: None,
                        position: Position::default(),
                    }),
                ],
            },
        })]
    );
}

#[test]
fn if_errors_point_at_the_source() {
    let error = parse_file("component A { Container { @if x { } } }").unwrap_err();
    assert_eq!(error.message(), "expected '('");
    assert_eq!((error.line(), error.column()), (1, 31));

    let error = parse_file("component A { Container { @if (x) } } }").unwrap_err();
    assert_eq!(error.message(), "expected '{'");

    let error =
        parse_file("component A { Container { @if (x) { } else Container { } } }").unwrap_err();
    assert_eq!(
        error.message(),
        "expected '@else': block constructs start with '@'"
    );
}

#[test]
fn block_constructs_require_the_at_marker() {
    let error = parse_file("component A { Container { if (x) { } } }").unwrap_err();
    assert_eq!(
        error.message(),
        "expected '@if': block constructs start with '@'"
    );
    assert_eq!((error.line(), error.column()), (1, 27));

    let error = parse_file("component A { Container { each (items as item) { } } }").unwrap_err();
    assert_eq!(
        error.message(),
        "expected '@each': block constructs start with '@'"
    );
    assert_eq!((error.line(), error.column()), (1, 27));

    let error = parse_file("component A { Container { slot } }").unwrap_err();
    assert_eq!(
        error.message(),
        "expected '@slot': block constructs start with '@'"
    );
}

#[test]
fn slots_parse() {
    let file = parse_file("component Card { Container { @slot } }").unwrap();
    let component = file.declaration.as_ref().unwrap();
    let Member::Element(root) = &component.members[0] else {
        panic!("expected a root element");
    };
    let [BlockItem::Slot(slot)] = root.block.items.as_slice() else {
        panic!("expected a slot");
    };
    assert_eq!(slot.name, None);
    assert!(slot.block.is_none());

    let file = parse_file("component Card { Container { @slot(header) } }").unwrap();
    let component = file.declaration.as_ref().unwrap();
    let Member::Element(root) = &component.members[0] else {
        panic!("expected a root element");
    };
    let [BlockItem::Slot(slot)] = root.block.items.as_slice() else {
        panic!("expected a slot");
    };
    assert_eq!(slot.name.as_deref(), Some("header"));
    assert!(slot.block.is_none());

    let file =
        parse_file("component Card { Container { @slot(header) { Text { \"x\" } } } }").unwrap();
    let component = file.declaration.as_ref().unwrap();
    let Member::Element(root) = &component.members[0] else {
        panic!("expected a root element");
    };
    let [BlockItem::Slot(slot)] = root.block.items.as_slice() else {
        panic!("expected a slot");
    };
    assert_eq!(slot.name.as_deref(), Some("header"));
    assert!(slot.block.is_some());
}

#[test]
fn nested_errors_point_at_the_source() {
    let error = parse_file("component A { Container { Container } }").unwrap_err();
    assert_eq!(error.message(), "expected '{'");
    assert_eq!((error.line(), error.column()), (1, 37));

    let error = parse_file("component A { Container { Container { } }").unwrap_err();
    assert_eq!(error.message(), "expected '}'");
    assert_eq!((error.line(), error.column()), (1, 42));

    let error = parse_file("component A { Container { Text { } } }").unwrap_err();
    assert_eq!(error.message(), "expected expression");
    assert_eq!((error.line(), error.column()), (1, 34));

    let error = parse_file("component A { Container { Button (on: x) { } } }").unwrap_err();
    assert_eq!(error.message(), "expected name");
    assert_eq!((error.line(), error.column()), (1, 37));
}

#[test]
fn leaf_blocks_hold_one_expression() {
    let source = r#"component Item {
        Container {
            Text (text-main) { "Hello " + name }
            Image (item-image) { icon }
            Text { `Hi ${name}` }
        }
    }"#;
    let file = parse_file(source).unwrap();
    let component = file.declaration.as_ref().unwrap();
    let text = |block: Block| {
        BlockItem::Element(Element {
            position: Position::default(),
            name: "Text".to_owned(),
            widget: Some(Widget::Text),
            args: Vec::new(),
            block,
        })
    };
    assert_eq!(
        component.members,
        vec![Member::Element(Element {
            position: Position::default(),
            name: "Container".to_owned(),
            widget: Some(Widget::Container),
            args: Vec::new(),
            block: Block {
                items: vec![
                    BlockItem::Element(Element {
                        position: Position::default(),
                        name: "Text".to_owned(),
                        widget: Some(Widget::Text),
                        args: vec![Arg::Class("text-main".to_owned())],
                        block: Block {
                            items: vec![BlockItem::Expr(expr(ExprKind::Binary {
                                op: BinaryOp::Add,
                                left: Box::new(string("Hello ")),
                                right: Box::new(ident("name")),
                            }))],
                        },
                    }),
                    BlockItem::Element(Element {
                        position: Position::default(),
                        name: "Image".to_owned(),
                        widget: Some(Widget::Image),
                        args: vec![Arg::Class("item-image".to_owned())],
                        block: Block {
                            items: vec![BlockItem::Expr(ident("icon"))],
                        },
                    }),
                    text(Block {
                        items: vec![BlockItem::Expr(expr(ExprKind::Template(vec![
                            TemplatePart::Text("Hi ".to_owned()),
                            TemplatePart::Expr(Box::new(ident("name"))),
                        ])))],
                    }),
                ],
            },
        })]
    );
}

#[test]
fn leaf_block_errors_point_at_the_source() {
    let error = parse_file("component A { Text { } }").unwrap_err();
    assert_eq!(error.message(), "expected expression");

    let error = parse_file("component A { Text { \"a\" \"b\" } }").unwrap_err();
    assert_eq!(error.message(), "expected '}'");

    let error = parse_file("component A { Text { Container { } } }").unwrap_err();
    assert_eq!(
        error.message(),
        "Text is a leaf widget: nested elements are not allowed"
    );
    assert_eq!((error.line(), error.column()), (1, 32));

    let error = parse_file("component A { Image { Container (item-image) { } } }").unwrap_err();
    assert_eq!(
        error.message(),
        "Image is a leaf widget: nested elements are not allowed"
    );
    assert_eq!((error.line(), error.column()), (1, 46));

    let error = parse_file("component A { Container { Text { Button { } } } }").unwrap_err();
    assert_eq!(
        error.message(),
        "Text is a leaf widget: nested elements are not allowed"
    );
    assert_eq!((error.line(), error.column()), (1, 41));
}

#[test]
fn each_blocks_parse() {
    let source = r#"component List {
        Container {
            @each (items as item) { Container { } }
            @each (items as item, idx) { Container (number: idx) { } }
            @each (items as item (item.id)) { Container { } }
            @each (groups[0] as item, idx (item.id)) { Container { } }
            @each (items as item) { Container { } } @else { Container { } }
        }
    }"#;
    let file = parse_file(source).unwrap();
    let component = file.declaration.as_ref().unwrap();
    let element = |name: &str| Element {
        position: Position::default(),
        name: name.to_owned(),
        widget: Widget::from_name(name),
        args: Vec::new(),
        block: Block { items: Vec::new() },
    };
    let row = || BlockItem::Element(element("Container"));
    let key = || {
        Some(expr(ExprKind::Field {
            target: Box::new(ident("item")),
            name: "id".to_owned(),
        }))
    };
    assert_eq!(
        component.members,
        vec![Member::Element(Element {
            position: Position::default(),
            name: "Container".to_owned(),
            widget: Some(Widget::Container),
            args: Vec::new(),
            block: Block {
                items: vec![
                    BlockItem::Each(EachBlock {
                        collection: ident("items"),
                        item: "item".to_owned(),
                        index: None,
                        key: None,
                        block: Block { items: vec![row()] },
                        else_block: None,
                        position: Position::default(),
                    }),
                    BlockItem::Each(EachBlock {
                        collection: ident("items"),
                        item: "item".to_owned(),
                        index: Some("idx".to_owned()),
                        key: None,
                        block: Block {
                            items: vec![BlockItem::Element(Element {
                                position: Position::default(),
                                name: "Container".to_owned(),
                                widget: Some(Widget::Container),
                                args: vec![Arg::Named {
                                    name: "number".to_owned(),
                                    value: ident("idx"),
                                }],
                                block: Block { items: Vec::new() },
                            })],
                        },
                        else_block: None,
                        position: Position::default(),
                    }),
                    BlockItem::Each(EachBlock {
                        collection: ident("items"),
                        item: "item".to_owned(),
                        index: None,
                        key: key(),
                        block: Block { items: vec![row()] },
                        else_block: None,
                        position: Position::default(),
                    }),
                    BlockItem::Each(EachBlock {
                        collection: expr(ExprKind::Index {
                            target: Box::new(ident("groups")),
                            index: Box::new(number(0.0)),
                        }),
                        item: "item".to_owned(),
                        index: Some("idx".to_owned()),
                        key: key(),
                        block: Block { items: vec![row()] },
                        else_block: None,
                        position: Position::default(),
                    }),
                    BlockItem::Each(EachBlock {
                        collection: ident("items"),
                        item: "item".to_owned(),
                        index: None,
                        key: None,
                        block: Block { items: vec![row()] },
                        else_block: Some(Block {
                            items: vec![BlockItem::Element(element("Container"))],
                        }),
                        position: Position::default(),
                    }),
                ],
            },
        })]
    );
}

#[test]
fn each_errors_point_at_the_source() {
    let error = parse_file("component A { Container { @each items as item { } } }").unwrap_err();
    assert_eq!(error.message(), "expected '('");
    assert_eq!((error.line(), error.column()), (1, 33));

    let error = parse_file("component A { Container { @each (items item) { } } }").unwrap_err();
    assert_eq!(error.message(), "expected 'as'");

    let error =
        parse_file("component A { Container { @each (items as item, ) { } } }").unwrap_err();
    assert_eq!(error.message(), "expected name");

    let error =
        parse_file("component A { Container { @each (items as item (item.id { } } }").unwrap_err();
    assert_eq!(error.message(), "expected ')'");

    let error = parse_file("component A { Container { @each (items as item) } }").unwrap_err();
    assert_eq!(error.message(), "expected '{'");
}

#[test]
fn directive_nodes_keep_source_positions() {
    let source = "\
component A {
    Container {
        @if (done) { }
        @else if (retry) { @each (items as item) { } }
    }
}";
    let component = parse_file(source).unwrap().declaration.unwrap();
    let Member::Element(container) = &component.members[0] else {
        panic!("expected container element");
    };
    let BlockItem::If(if_block) = &container.block.items[0] else {
        panic!("expected if block");
    };
    assert_eq!(if_block.position, Position { line: 3, column: 9 });
    assert_eq!(
        if_block.branches[0].position,
        Position {
            line: 3,
            column: 13
        }
    );
    assert_eq!(
        if_block.branches[1].position,
        Position {
            line: 4,
            column: 18
        }
    );

    let BlockItem::Each(each) = &if_block.branches[1].block.items[0] else {
        panic!("expected each block");
    };
    assert_eq!(
        each.position,
        Position {
            line: 4,
            column: 28
        }
    );
}
