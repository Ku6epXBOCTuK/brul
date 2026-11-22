use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use brul_widget_schema::{ELEMENTS, ElementKind};

pub fn generate(out_dir: &Path) {
    let mut source = String::from(
        "#[derive(Debug, Clone, Copy, PartialEq, Eq)]\n\
         pub enum Widget {\n",
    );
    for element in ELEMENTS {
        writeln!(source, "    {},", element.name).expect("cannot write a widget variant");
    }
    source.push_str("}\n\nimpl Widget {\n");
    source.push_str("    pub const ALL: &'static [Widget] = &[\n");
    for element in ELEMENTS {
        writeln!(source, "        Widget::{},", element.name).expect("cannot write a widget entry");
    }
    source.push_str("    ];\n\n");
    source.push_str("    pub fn from_name(name: &str) -> Option<Self> {\n");
    source.push_str("        match name {\n");
    for element in ELEMENTS {
        writeln!(
            source,
            "            {:?} => Some(Widget::{}),",
            element.name, element.name
        )
        .expect("cannot write a widget name");
    }
    source.push_str("            _ => None,\n        }\n    }\n\n");
    source.push_str("    pub fn name(self) -> &'static str {\n");
    source.push_str("        match self {\n");
    for element in ELEMENTS {
        writeln!(
            source,
            "            Widget::{} => {:?},",
            element.name, element.name
        )
        .expect("cannot write a widget name");
    }
    source.push_str("        }\n    }\n\n");
    source.push_str("    pub fn is_leaf(self) -> bool {\n");
    source.push_str("        match self {\n");
    for element in ELEMENTS {
        writeln!(
            source,
            "            Widget::{} => {},",
            element.name,
            matches!(element.kind, ElementKind::Leaf)
        )
        .expect("cannot write a widget kind");
    }
    source.push_str("        }\n    }\n}\n");
    fs::write(out_dir.join("widget.rs"), source).expect("cannot write widget.rs");
}
