pub use brul_script::builtins::{len, number, string};

include!(concat!(env!("OUT_DIR"), "/lang_builtin_markers.rs"));
