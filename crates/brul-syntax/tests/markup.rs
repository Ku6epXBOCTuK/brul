use brul_syntax::ast::{BinaryOp, Expr, TemplatePart, UnaryOp};
use brul_syntax::markup::{
    Arg, Block, BlockItem, Component, Declaration, EachBlock, Element, Event, EventParam, File,
    IfBlock, Member, Prop, PropType, Window, parse_file,
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
    let source = "component CheckRow { Row { Text { \"hi\" } View { View { } } } }";
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
                        block: Block {
                            items: vec![BlockItem::Expr(Expr::String("hi".to_owned()))],
                        },
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

#[test]
fn if_else_blocks_parse() {
    let source = r#"component CheckRow {
        Row {
            if (done) { Span { } } else { View { } }
            if (failed) { Text { "no" } }
        }
    }"#;
    let file = parse_file(source).unwrap();
    let Declaration::Component(component) = &file.declarations[0] else {
        panic!("expected a component");
    };
    let element = |name: &str| Element {
        name: name.to_owned(),
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
                    BlockItem::If(IfBlock {
                        condition: Expr::Ident("done".to_owned()),
                        then_block: Block {
                            items: vec![BlockItem::Element(element("Span"))],
                        },
                        else_block: Some(Block {
                            items: vec![BlockItem::Element(element("View"))],
                        }),
                    }),
                    BlockItem::If(IfBlock {
                        condition: Expr::Ident("failed".to_owned()),
                        then_block: Block {
                            items: vec![BlockItem::Element(Element {
                                name: "Text".to_owned(),
                                args: Vec::new(),
                                block: Block {
                                    items: vec![BlockItem::Expr(Expr::String("no".to_owned()))],
                                },
                            })],
                        },
                        else_block: None,
                    }),
                ],
            },
        })]
    );
}

#[test]
fn if_errors_point_at_the_source() {
    let error = parse_file("component A { Row { if x { } } }").unwrap_err();
    assert_eq!(error.message, "expected '('");
    assert_eq!((error.line, error.column), (1, 24));

    let error = parse_file("component A { Row { if (x) } } }").unwrap_err();
    assert_eq!(error.message, "expected '{'");
    assert_eq!((error.line, error.column), (1, 28));

    let error = parse_file("component A { Row { if (x) { } else View { } } }").unwrap_err();
    assert_eq!(error.message, "expected '{'");
}

#[test]
fn nested_errors_point_at_the_source() {
    let error = parse_file("component A { Row { View } }").unwrap_err();
    assert_eq!(error.message, "expected '{'");
    assert_eq!((error.line, error.column), (1, 26));

    let error = parse_file("component A { Row { View { } }").unwrap_err();
    assert_eq!(error.message, "expected '}'");
    assert_eq!((error.line, error.column), (1, 31));

    let error = parse_file("component A { Row { Text { } } }").unwrap_err();
    assert_eq!(error.message, "expected expression");
    assert_eq!((error.line, error.column), (1, 28));

    let error = parse_file("component A { Row { Button (on: x) { } } }").unwrap_err();
    assert_eq!(error.message, "expected name");
    assert_eq!((error.line, error.column), (1, 31));
}

#[test]
fn leaf_blocks_hold_one_expression() {
    let source = r#"component Item {
        Row {
            Text (text-main) { "Hello " + name }
            Image (item-image) { icon }
            Text { `Hi ${name}` }
        }
    }"#;
    let file = parse_file(source).unwrap();
    let Declaration::Component(component) = &file.declarations[0] else {
        panic!("expected a component");
    };
    let text = |block: Block| {
        BlockItem::Element(Element {
            name: "Text".to_owned(),
            args: Vec::new(),
            block,
        })
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
                        args: vec![Arg::Class("text-main".to_owned())],
                        block: Block {
                            items: vec![BlockItem::Expr(Expr::Binary {
                                op: BinaryOp::Add,
                                left: Box::new(Expr::String("Hello ".to_owned())),
                                right: Box::new(Expr::Ident("name".to_owned())),
                            })],
                        },
                    }),
                    BlockItem::Element(Element {
                        name: "Image".to_owned(),
                        args: vec![Arg::Class("item-image".to_owned())],
                        block: Block {
                            items: vec![BlockItem::Expr(Expr::Ident("icon".to_owned()))],
                        },
                    }),
                    text(Block {
                        items: vec![BlockItem::Expr(Expr::Template(vec![
                            TemplatePart::Text("Hi ".to_owned()),
                            TemplatePart::Expr(Box::new(Expr::Ident("name".to_owned()))),
                        ]))],
                    }),
                ],
            },
        })]
    );
}

#[test]
fn leaf_block_errors_point_at_the_source() {
    let error = parse_file("component A { Text { } }").unwrap_err();
    assert_eq!(error.message, "expected expression");

    let error = parse_file("component A { Text { \"a\" \"b\" } }").unwrap_err();
    assert_eq!(error.message, "expected '}'");

    let error = parse_file("component A { Text { View { } } }").unwrap_err();
    assert_eq!(
        error.message,
        "Text is a leaf widget: nested elements are not allowed"
    );
    assert_eq!((error.line, error.column), (1, 27));

    let error = parse_file("component A { Image { Row (item-image) { } } }").unwrap_err();
    assert_eq!(
        error.message,
        "Image is a leaf widget: nested elements are not allowed"
    );
    assert_eq!((error.line, error.column), (1, 40));

    let error = parse_file("component A { Row { Text { Button { } } } }").unwrap_err();
    assert_eq!(
        error.message,
        "Text is a leaf widget: nested elements are not allowed"
    );
    assert_eq!((error.line, error.column), (1, 35));
}

#[test]
fn each_blocks_parse() {
    let source = r#"component List {
        Column {
            each (items as item) { Row { } }
            each (items as item, idx) { Row (number: idx) { } }
            each (items as item (item.id)) { Row { } }
            each (groups[0] as item, idx (item.id)) { Row { } }
        }
    }"#;
    let file = parse_file(source).unwrap();
    let Declaration::Component(component) = &file.declarations[0] else {
        panic!("expected a component");
    };
    let element = |name: &str| Element {
        name: name.to_owned(),
        args: Vec::new(),
        block: Block { items: Vec::new() },
    };
    let row = || BlockItem::Element(element("Row"));
    let key = || {
        Some(Expr::Field {
            target: Box::new(Expr::Ident("item".to_owned())),
            name: "id".to_owned(),
        })
    };
    assert_eq!(
        component.members,
        vec![Member::Element(Element {
            name: "Column".to_owned(),
            args: Vec::new(),
            block: Block {
                items: vec![
                    BlockItem::Each(EachBlock {
                        collection: Expr::Ident("items".to_owned()),
                        item: "item".to_owned(),
                        index: None,
                        key: None,
                        block: Block { items: vec![row()] },
                    }),
                    BlockItem::Each(EachBlock {
                        collection: Expr::Ident("items".to_owned()),
                        item: "item".to_owned(),
                        index: Some("idx".to_owned()),
                        key: None,
                        block: Block {
                            items: vec![BlockItem::Element(Element {
                                name: "Row".to_owned(),
                                args: vec![Arg::Named {
                                    name: "number".to_owned(),
                                    value: Expr::Ident("idx".to_owned()),
                                }],
                                block: Block { items: Vec::new() },
                            })],
                        },
                    }),
                    BlockItem::Each(EachBlock {
                        collection: Expr::Ident("items".to_owned()),
                        item: "item".to_owned(),
                        index: None,
                        key: key(),
                        block: Block { items: vec![row()] },
                    }),
                    BlockItem::Each(EachBlock {
                        collection: Expr::Index {
                            target: Box::new(Expr::Ident("groups".to_owned())),
                            index: Box::new(Expr::Number(0.0)),
                        },
                        item: "item".to_owned(),
                        index: Some("idx".to_owned()),
                        key: key(),
                        block: Block { items: vec![row()] },
                    }),
                ],
            },
        })]
    );
}

#[test]
fn each_errors_point_at_the_source() {
    let error = parse_file("component A { Row { each items as item { } } }").unwrap_err();
    assert_eq!(error.message, "expected '('");
    assert_eq!((error.line, error.column), (1, 26));

    let error = parse_file("component A { Row { each (items item) { } } }").unwrap_err();
    assert_eq!(error.message, "expected 'as'");

    let error = parse_file("component A { Row { each (items as item, ) { } } }").unwrap_err();
    assert_eq!(error.message, "expected name");

    let error = parse_file("component A { Row { each (items as item (item.id { } } }").unwrap_err();
    assert_eq!(error.message, "expected ')'");

    let error = parse_file("component A { Row { each (items as item) } }").unwrap_err();
    assert_eq!(error.message, "expected '{'");
}
