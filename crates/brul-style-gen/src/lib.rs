mod codegen;
mod docs;
mod layout;
mod style;
mod syntax;
mod text;
mod tokens;
mod util;
mod widget;

use std::fs;
use std::path::Path;

use crate::codegen::codegen_fields;
use crate::layout::layout_style;
use crate::style::codegen;
use crate::style::style;
use crate::syntax::syntax;
use crate::text::text;
use crate::widget::widget;

pub use docs::properties_doc;

pub enum Output {
    Style,
    Syntax,
    Layout,
    Widget,
    Text,
    Codegen,
    CodegenFields,
}

pub fn generate(output: Output, out_dir: &Path) {
    let (file, tokens) = match output {
        Output::Style => ("style_properties.rs", style()),
        Output::Syntax => ("style_shorthands.rs", syntax()),
        Output::Layout => ("style_layout.rs", layout_style()),
        Output::Widget => ("style_widget.rs", widget()),
        Output::Text => ("style_text.rs", text()),
        Output::Codegen => ("style_kind.rs", codegen()),
        Output::CodegenFields => ("style_fields.rs", codegen_fields()),
    };
    let parsed = syn::parse2::<syn::File>(tokens).expect("generated style code parses");
    fs::write(out_dir.join(file), prettyplease::unparse(&parsed))
        .expect("cannot write generated style code");
}
