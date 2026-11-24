use std::collections::HashSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StateKey {
    pub(crate) module: String,
    pub(crate) path: String,
}

impl StateKey {
    pub(crate) fn literal(&self) -> String {
        literal(&self.module, &self.path)
    }
}

#[derive(Default)]
pub(crate) struct StateIds {
    seen: HashSet<(String, String)>,
}

impl StateIds {
    pub(crate) fn register(&mut self, module: &str, path: &str) -> Result<StateKey, String> {
        if !self.seen.insert((module.to_owned(), path.to_owned())) {
            return Err(format!("duplicate state key '{module}/{path}'"));
        }
        Ok(StateKey {
            module: module.to_owned(),
            path: path.to_owned(),
        })
    }
}

fn literal(module: &str, path: &str) -> String {
    format!("brul::StateId::source({module:?}, {path:?})")
}

pub(crate) fn child(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_owned()
    } else {
        format!("{path}/{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_source_and_slot_gets_its_own_id() {
        let first = StateKey {
            module: "module".into(),
            path: child("Widget", "attach:editor"),
        };
        let second = StateKey {
            module: "module".into(),
            path: child("Widget", "attach:keyboard"),
        };
        let other_widget = StateKey {
            module: "module".into(),
            path: child("Other", "attach:editor"),
        };
        let other_module = StateKey {
            module: "other".into(),
            path: child("Widget", "attach:editor"),
        };

        assert_ne!(first.literal(), second.literal());
        assert_ne!(first.literal(), other_widget.literal());
        assert_ne!(first.literal(), other_module.literal());
        assert_eq!(
            first.literal(),
            StateKey {
                module: "module".into(),
                path: child("Widget", "attach:editor"),
            }
            .literal()
        );
    }

    #[test]
    fn a_duplicate_key_is_rejected() {
        let mut ids = StateIds::default();
        ids.register("module", "Widget/attach:editor")
            .expect("first");
        assert_eq!(
            ids.register("module", "Widget/attach:editor"),
            Err("duplicate state key 'module/Widget/attach:editor'".to_owned())
        );
    }
}
