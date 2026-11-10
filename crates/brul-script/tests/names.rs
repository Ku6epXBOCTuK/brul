use brul_script::{Names, VariableId};

#[test]
fn ids_follow_first_appearance() {
    let mut names = Names::new();
    assert_eq!(names.intern("a").index(), 0);
    assert_eq!(names.intern("b").index(), 1);
    assert_eq!(names.intern("a").index(), 0);
    assert_eq!(names.len(), 2);
    assert_eq!(names.names(), &["a", "b"][..]);
}

#[test]
fn name_maps_back_to_id() {
    let mut names = Names::new();
    let id = names.intern("selected");
    assert_eq!(names.get("selected"), Some(id));
    assert_eq!(names.get("missing"), None);
    assert_eq!(names.name(id), Some("selected"));
    assert_eq!(names.name(VariableId::from_index(3)), None);
}

#[test]
fn empty_table_has_no_names() {
    let names: Names<'static> = Names::new();
    assert!(names.is_empty());
    assert_eq!(names.len(), 0);
}
