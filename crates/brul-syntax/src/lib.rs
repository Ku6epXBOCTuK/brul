//! `brul-syntax`: SFC splitter and parsers of BRUL languages into the AST.

macro_rules! rule {
    ($vis:vis enum $name:ident { $($variant:ident),* $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $vis enum $name {
            $($variant),*
        }

        impl $name {
            pub const ALL: &[$name] = &[$($name::$variant),*];

            pub fn name(self) -> &'static str {
                const NAMES: &[&str] = &[$(stringify!($variant)),*];
                NAMES[self as usize]
            }
        }
    };
}

pub mod ast;
#[path = "internal/cursor.rs"]
pub mod cursor;
pub mod ebnf;
#[path = "internal/error.rs"]
pub mod error;
pub mod lexer;
pub mod markup;
pub mod parser;
pub mod script;
pub mod sfc;
pub mod style;
#[path = "internal/types.rs"]
pub mod types;
pub mod uses;

pub use types::TypeName;
