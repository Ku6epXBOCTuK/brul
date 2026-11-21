#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TypeSpec {
    pub variant: &'static str,
    pub name: &'static str,
    pub payload: Option<&'static str>,
    pub rust: &'static str,
    pub local: &'static str,
    pub dyn_wrap: &'static str,
    pub dyn_local: &'static str,
    pub convert: &'static str,
    pub from_dyn: Option<&'static str>,
    pub clone_on_read: bool,
    pub is_value: bool,
    pub state: bool,
    pub state_read: &'static str,
    pub state_write: &'static str,
    pub default: &'static str,
    pub prop_expression: &'static str,
    pub value_names: &'static [&'static str],
    pub literal: &'static str,
    pub note: &'static str,
}

const BASE: TypeSpec = TypeSpec {
    variant: "",
    name: "",
    payload: None,
    rust: "brul::Dyn",
    local: "brul::Dyn",
    dyn_wrap: "{code}",
    dyn_local: "{code}",
    convert: "{code}",
    from_dyn: None,
    clone_on_read: false,
    is_value: false,
    state: false,
    state_read: "",
    state_write: "",
    default: "brul::Dyn::null()",
    prop_expression: "Image",
    value_names: &[],
    literal: "-",
    note: "",
};

pub const TYPES: &[TypeSpec] = &[
    TypeSpec {
        variant: "Bool",
        name: "Bool",
        rust: "bool",
        local: "bool",
        dyn_wrap: "brul::Dyn::flag({code})",
        dyn_local: "brul::Dyn::flag({code})",
        convert: "(&{code}).try_into()?",
        from_dyn: Some("bool::try_from"),
        state: true,
        state_read: "brul::Dyn::flag({state}.get())",
        state_write: "{state}.set((&{value}).try_into()?);",
        default: "false",
        prop_expression: "Flag",
        value_names: &["Bool"],
        literal: "`true`, `false`",
        note: "boolean flag",
        ..BASE
    },
    TypeSpec {
        variant: "Number",
        name: "Number",
        rust: "f64",
        local: "f64",
        dyn_wrap: "brul::Dyn::num({code})",
        dyn_local: "brul::Dyn::num({code})",
        convert: "(&{code}).try_into()?",
        from_dyn: Some("f64::try_from"),
        state: true,
        state_read: "brul::Dyn::num({state}.get())",
        state_write: "{state}.set((&{value}).try_into()?);",
        default: "0.0",
        prop_expression: "Number",
        value_names: &["Number"],
        literal: "`42`, `3.14`, `-1`",
        note: "f64 in generated code",
        ..BASE
    },
    TypeSpec {
        variant: "String",
        name: "String",
        rust: "String",
        local: "String",
        dyn_wrap: "brul::Dyn::text({code})",
        dyn_local: "brul::Dyn::text({code}.as_str())",
        convert: "(&{code}).try_into()?",
        from_dyn: Some("String::try_from"),
        clone_on_read: true,
        state: true,
        state_read: "brul::Dyn::text({state}.get().as_str())",
        state_write: "{state}.set((&{value}).try_into()?);",
        default: "String::new()",
        prop_expression: "Text",
        value_names: &["String"],
        literal: "\"text\", `template ${expr}`",
        note: "string templates are needed for Text",
        ..BASE
    },
    TypeSpec {
        variant: "Image",
        name: "Image",
        dyn_wrap: "{code}",
        value_names: &["Image", "Null"],
        literal: "`empty`",
        note: "empty image; a path creates an application",
        ..BASE
    },
    TypeSpec {
        variant: "Null",
        name: "Null",
        local: "()",
        dyn_wrap: "brul::Dyn::null()",
        dyn_local: "brul::Dyn::null()",
        convert: "({code}).unit()?",
        from_dyn: Some("<()>::try_from"),
        state: true,
        state_read: "brul::Dyn::null()",
        state_write: "{ let _ = &{value}; {state}.set(()); }",
        default: "()",
        value_names: &["Null"],
        literal: "`null`",
        note: "no value",
        ..BASE
    },
    TypeSpec {
        variant: "Object",
        name: "Object",
        clone_on_read: true,
        is_value: true,
        value_names: &["Object"],
        literal: "`{ name: \"...\" }`",
        note: "models for `each` and configurations",
        ..BASE
    },
    TypeSpec {
        variant: "Timer",
        name: "Timer",
        rust: "brul::TimerId",
        local: "brul::TimerId",
        dyn_wrap: "brul::Dyn::from({code})",
        dyn_local: "brul::Dyn::from({code})",
        convert: "(&{code}).try_into()?",
        value_names: &["Timer"],
        literal: "descriptor of `every` / `after`",
        note: "canceled with `cancel`",
        ..BASE
    },
    TypeSpec {
        variant: "Void",
        name: "Void",
        dyn_wrap: "brul::Dyn::null()",
        dyn_local: "brul::Dyn::null()",
        literal: "-",
        note: "function return only",
        ..BASE
    },
    TypeSpec {
        variant: "Array",
        name: "Array",
        payload: Some("Box<Type>"),
        clone_on_read: true,
        is_value: true,
        value_names: &["Array"],
        literal: "`[a, b, c]`",
        note: "`Array<T>`; empty array takes its type from the annotation",
        ..BASE
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_type_has_a_unique_name_and_variant() {
        let mut names: Vec<_> = TYPES.iter().map(|spec| spec.name).collect();
        names.sort_unstable();
        let mut variants: Vec<_> = TYPES.iter().map(|spec| spec.variant).collect();
        variants.sort_unstable();
        for values in [names, variants] {
            let mut unique = values.clone();
            unique.dedup();
            assert_eq!(unique.len(), values.len());
        }
    }

    #[test]
    fn every_type_has_complete_mappings() {
        for spec in TYPES {
            for (field, value) in [
                ("variant", spec.variant),
                ("name", spec.name),
                ("rust", spec.rust),
                ("local", spec.local),
                ("dyn_wrap", spec.dyn_wrap),
                ("dyn_local", spec.dyn_local),
                ("convert", spec.convert),
                ("default", spec.default),
                ("prop_expression", spec.prop_expression),
                ("literal", spec.literal),
                ("note", spec.note),
            ] {
                assert!(!value.is_empty(), "{} needs {field}", spec.name);
            }
            if spec.state {
                assert!(
                    !spec.state_read.is_empty(),
                    "{} needs state_read",
                    spec.name
                );
                assert!(
                    spec.state_write.contains("{state}") && spec.state_write.contains("{value}"),
                    "{} has an incomplete state_write",
                    spec.name
                );
            }
            if spec.name != "Void" {
                assert!(
                    !spec.value_names.is_empty(),
                    "{} needs value names",
                    spec.name
                );
            }
        }
    }
}
