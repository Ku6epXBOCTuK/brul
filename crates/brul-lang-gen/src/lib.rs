mod builtins;
mod docs;
mod types;

pub use docs::types_doc;

use std::fs;
use std::path::Path;

use proc_macro2::TokenStream;

pub enum Output {
    Types,
    Builtins,
}

pub fn generate(output: Output, out_dir: &Path) {
    let (file, tokens): (&str, TokenStream) = match output {
        Output::Types => ("lang_types.rs", types::types()),
        Output::Builtins => ("lang_builtins.rs", builtins::builtins()),
    };
    let parsed = syn::parse2::<syn::File>(tokens).expect("generated language code parses");
    fs::write(out_dir.join(file), prettyplease::unparse(&parsed))
        .expect("cannot write generated language code");
}
