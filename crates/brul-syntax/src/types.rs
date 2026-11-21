pub trait TypeName {
    fn type_name(&self) -> &'static str;
}

include!(concat!(env!("OUT_DIR"), "/lang_types.rs"));

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
