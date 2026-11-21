use std::fmt;

pub trait TypeName {
    fn type_name(&self) -> &'static str;
}

macro_rules! types {
    (
        $( $simple:ident ),+ $(,)? ;
        $( $data:ident ($payload:ty) => $name:literal ),+ $(,)?
    ) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum Type {
            $( $simple, )+
            $( $data($payload), )+
        }

        impl Type {
            pub fn from_name(name: &str) -> Option<Self> {
                Some(match name {
                    $( stringify!($simple) => Self::$simple, )+
                    _ => return None,
                })
            }
        }

        impl TypeName for Type {
            fn type_name(&self) -> &'static str {
                match self {
                    $( Self::$simple => stringify!($simple), )+
                    $( Self::$data(_) => $name, )+
                }
            }
        }
    };
}

types! {
    Bool, Number, String, Image, Null, Object, Timer, Void;
    Array(Box<Type>) => "Array",
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Array(item) => write!(f, "PropType::Array(Box::new({item}))"),
            other => write!(f, "PropType::{}", other.type_name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_type_has_its_own_name() {
        let names: Vec<&str> = [
            Type::Bool,
            Type::Number,
            Type::String,
            Type::Image,
            Type::Null,
            Type::Object,
            Type::Timer,
            Type::Void,
            Type::Array(Box::new(Type::Void)),
        ]
        .iter()
        .map(TypeName::type_name)
        .collect();
        assert!(names.iter().all(|name| !name.is_empty()));
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len());
    }

    #[test]
    fn from_name_round_trips_simple_types() {
        let simple = [
            Type::Bool,
            Type::Number,
            Type::String,
            Type::Image,
            Type::Null,
            Type::Object,
            Type::Timer,
            Type::Void,
        ];
        for ty in simple {
            assert_eq!(Type::from_name(ty.type_name()), Some(ty));
        }
        assert_eq!(Type::from_name("Array"), None);
    }
}
