use super::*;
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
