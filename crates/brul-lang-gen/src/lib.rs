mod builtins;
mod docs;
mod types;

pub use docs::{builtins_doc, types_doc};

use std::fs;
use std::path::Path;

use proc_macro2::TokenStream;

pub enum Output {
    Types,
    Builtins,
    Dispatch,
    Markers,
    HostDispatch,
    ScriptCall,
}

pub fn generate(output: Output, out_dir: &Path) {
    let (file, tokens): (&str, TokenStream) = match output {
        Output::Types => ("lang_types.rs", types::types()),
        Output::Builtins => ("lang_builtins.rs", builtins::builtins()),
        Output::Dispatch => ("lang_builtin_dispatch.rs", builtins::dispatch()),
        Output::Markers => ("lang_builtin_markers.rs", builtins::markers()),
        Output::HostDispatch => ("lang_builtin_host.rs", builtins::host_dispatch()),
        Output::ScriptCall => ("lang_script_call.rs", builtins::script_call()),
    };
    let parsed = syn::parse2::<syn::File>(tokens).expect("generated language code parses");
    fs::write(out_dir.join(file), prettyplease::unparse(&parsed))
        .expect("cannot write generated language code");
}
