use std::path::Path;
use std::path::PathBuf;

use brul_codegen::emit;
use brul_codegen::{Capabilities, commands, generate as generate_module, generate_with};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/codegen")
        .join(name)
}

fn generate(name: &str) -> Result<String, brul_syntax::error::Error> {
    let dir = fixture(name);
    let input = dir.join("input.brul");
    let commands_path = dir.join("commands.txt");
    let module = if commands_path.is_file() {
        let capabilities =
            Capabilities::discover_with_commands(&dir, commands::read(&commands_path)?)?;
        generate_with(&input, &capabilities)?
    } else {
        generate_module(&input, &dir)?
    };
    Ok(module.flat().to_owned())
}

#[test]
fn discovers_capabilities_for_explicit_generation() {
    let dir = fixture("emits_an_attachment_on_an_element");
    let input = dir.join("input.brul");
    let capabilities = Capabilities::discover(&dir).expect("capabilities");
    assert!(capabilities.commands.is_empty());
    assert!(capabilities.specs.contains_key("editor"));
    assert!(capabilities.specs.contains_key("keyboard"));
    assert!(capabilities.specs.contains_key("pointer"));
    assert!(capabilities.specs.contains_key("scroll"));
    assert!(capabilities.specs.contains_key("Video"));
    assert!(capabilities.specs.contains_key("WebView"));
    assert!(capabilities.specs.contains_key("Canvas"));
    let module = generate_with(&input, &capabilities).expect("generated module");
    let expected = include_str!("fixtures/codegen/emits_an_attachment_on_an_element/expected.rs");
    assert_eq!(module.flat(), expected);
}

#[test]
fn discovers_capabilities_for_an_entry_inside_src() {
    let dir = fixture("discovers_capabilities_from_a_src_entry");
    let input = dir.join("src/window.brul");
    let capabilities = Capabilities::discover(&dir).expect("capabilities");
    assert!(capabilities.specs.contains_key("custom_editor"));
    let module = generate_module(&input, &dir).expect("generated module");
    assert!(module.flat().contains("__brul_attach_custom_editor_apply"));
}

macro_rules! goldens {
    ($($name:ident),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                let actual = generate(stringify!($name)).expect("generated module");
                let expected = include_str!(concat!(
                    "fixtures/codegen/", stringify!($name), "/expected.rs"
                ));
                assert_eq!(actual, expected);
            }
        )+
    };
}

macro_rules! errors {
    ($($name:ident),+ $(,)?) => {
        $(
            #[test]
            fn $name() {
                let error = generate(stringify!($name)).expect_err("generation error");
                let expected = include_str!(concat!(
                    "fixtures/codegen/", stringify!($name), "/error.txt"
                ));
                assert_eq!(error.message(), expected.trim_end());
            }
        )+
    };
}

goldens! {
    a_component_file_resolves_its_imports,
    emits_a_cursor_property,
    emits_a_fragment_root,
    emits_an_attachment_on_an_element,
    emits_an_attachment_on_a_component,
    emits_an_attachment_on_a_window,
    emits_a_canvas_widget,
    emits_builtin_window_policies,
    emits_a_window_that_overrides_a_builtin_policy,
    emits_an_effect_binding,
    a_palette_without_themes_emits_an_empty_theme_id,
    a_window_icon_resource_reaches_the_window_config,
    accepts_host_calls_with_declared_variables_at_build,
    an_image_prop_accepts_a_resource_at_the_call_site,
    an_override_follows_the_component_rules,
    a_user_class_overrides_the_builtin_cursor,
    emits_a_dynamic_class_binding_and_its_table,
    emits_a_key_handler_on_an_element,
    emits_a_kit_input_with_props_and_events,
    emits_a_layer,
    emits_a_forward_anchor_target,
    emits_an_anchor_inside_each,
    emits_an_anchored_overlay,
    emits_a_registered_widget,
    emits_a_video_widget,
    emits_a_webview_widget,
    emits_a_linear_gradient_background,
    emits_a_modal_layer_that_dismisses,
    emits_a_multiline_text_input,
    emits_a_slot_inside_a_structure,
    emits_a_svg_resource,
    emits_a_text_input_with_its_binding_and_handlers,
    emits_a_theme_id_for_the_root_palette,
    emits_a_window_close_handler,
    emits_a_window_position,
    emits_a_window_position_2,
    emits_an_empty_slot_without_children,
    emits_an_if_chain_as_one_structure,
    emits_clip_only_when_it_is_set,
    emits_clip_only_when_it_is_set_2,
    emits_color_expressions,
    emits_component_events,
    emits_component_properties,
    emits_component_resource_with_component_prefix,
    emits_component_slot,
    emits_each_else_for_an_empty_collection,
    emits_each_inside_a_slot_fill,
    emits_each_structure,
    emits_every_window_with_its_flags,
    emits_geometry_shadow_and_text_properties_in_a_static_class,
    emits_if_structure,
    emits_image_value_binding,
    emits_imported_component,
    emits_named_slots,
    emits_named_fragment_slots,
    emits_names_in_first_appearance_order,
    emits_nested_each_scopes_with_shadowing,
    emits_no_class_table_without_a_dynamic_class,
    emits_parameter_bindings,
    emits_resource_and_image_with_tint,
    emits_script_program,
    emits_scripts_for_every_window,
    emits_scroll_only_when_it_is_set,
    emits_scroll_only_when_it_is_set_2,
    emits_static_tree,
    emits_text_binding,
    emits_text_styles_for_a_state,
    emits_the_focusable_flag_and_focus_handlers,
    emits_the_kit_components,
    emits_themes_and_states,
    emits_typed_builtin_calls,
    emits_typed_functions_and_direct_calls,
    emits_typed_local_scopes_with_inner_shadowing,
    emits_typed_return_statements,
    emits_vertical_alignment_for_a_text_state,
    emits_widget_handlers,
    emits_window_config,
    emits_window_handler,
    generates_a_component_file_without_a_window,
    imported_components_inherit_the_root_palette,
    resolves_an_import_that_declares_imports_of_its_own,
}

errors! {
    a_component_and_a_window_cannot_share_one_file,
    an_imported_component_cannot_declare_a_theme,
    an_override_of_a_component_that_is_not_imported_is_an_error,
    an_override_of_an_unknown_class_is_an_error,
    an_unknown_kit_component_lists_the_kit,
    an_unknown_theme_key_in_an_imported_component_names_the_file,
    rejects_a_focusable_that_is_not_a_literal,
    rejects_an_unknown_attachment,
    rejects_a_handler_that_ignores_the_event_arguments,
    rejects_a_multiline_that_is_not_a_literal,
    rejects_a_slot_outside_a_component,
    rejects_a_window_handler_that_ignores_the_key,
    rejects_a_window_used_as_a_component,
    rejects_assignment_to_local_constant_at_build,
    rejects_a_noop_key_event,
    rejects_broken_resources,
    rejects_broken_resources_2,
    rejects_broken_resources_3,
    rejects_a_class_on_a_component,
    rejects_a_slot_at_the_window_root,
    rejects_an_anchor_on_a_fragment,
    rejects_component_children,
    rejects_component_events,
    rejects_component_events_2,
    rejects_component_properties,
    rejects_component_properties_2,
    rejects_invalid_events,
    rejects_invalid_events_2,
    rejects_invalid_events_3,
    rejects_invalid_events_4,
    rejects_invalid_events_5,
    rejects_invalid_returns_at_build,
    rejects_invalid_returns_at_build_2,
    rejects_invalid_returns_at_build_3,
    rejects_malformed_slot_fills,
    rejects_malformed_slot_fills_2,
    rejects_malformed_slot_fills_3,
    rejects_malformed_slot_fills_4,
    rejects_old_theme_command_name_at_build,
    rejects_two_windows_with_the_same_name,
    rejects_undefined_script_variable_at_build,
    rejects_unknown_host_command_at_build,
    rejects_unknown_parameter,
    rejects_unknown_anchor_target,
    rejects_a_class_as_anchor_target,
    rejects_duplicate_anchor_name,
    rejects_anchor_with_offset,
    rejects_widget_children,
    rejects_widget_event,
    rejects_widget_property,
    rejects_wrong_host_argument_type_at_build,
    rejects_wrong_script_argument_type_at_build,
    themes_that_share_a_variant_are_an_error,
}

#[test]
fn emits_the_allocation_counter() {
    let module = generate("emits_the_allocation_counter").expect("module");
    let allocations = emit::allocations();
    assert!(module.contains("include!(concat!(env!(\"OUT_DIR\"), \"/allocations.rs\"));"));
    assert!(allocations.contains("#[cfg(brul_alloc_count)]\nmod brul_allocations {"));
    assert!(allocations.contains("#[global_allocator]"));
    assert!(allocations.contains("REALLOCATIONS.fetch_add(1, Ordering::Relaxed);"));
    assert!(
        module.contains("brul::perf::measure(brul_allocations::reset, brul_allocations::report);")
    );
}

#[test]
fn splits_every_component_into_its_own_file() {
    let dir = fixture("splits_every_component_into_its_own_file");
    let module = emit::split_module_file(&dir.join("input.brul")).unwrap();
    assert_eq!(module.files.len(), 1);
    let badge = &module.files[0];
    assert_eq!(badge.name, "bc_badge");
    assert!(badge.source.starts_with("use super::*;\n"));
    assert!(badge.source.contains("pub fn badge_view"));
    assert!(!badge.source.contains("pub fn demo_view("));
    assert!(module.root.contains("pub mod bc_badge {"));
    assert!(module.root.contains("pub use self::bc_badge::*;"));
    assert!(module.root.contains("pub fn demo_view("));
    assert_eq!(
        module.flat(),
        generate("splits_every_component_into_its_own_file").unwrap()
    );
}

#[test]
fn lists_imported_files_outside_the_entry_directory() {
    let dir = fixture("lists_imported_files_outside_the_entry_directory");
    let module = emit::split_module_file(&dir.join("app/window.brul")).unwrap();
    let badge = std::fs::canonicalize(dir.join("shared/badge.brul")).unwrap();
    let entry = std::fs::canonicalize(dir.join("app/window.brul")).unwrap();
    assert!(module.sources.contains(&badge), "{:?}", module.sources);
    assert!(module.sources.contains(&entry), "{:?}", module.sources);
}

#[test]
fn stages_component_and_root_resources_for_out_dir() {
    let root =
        emit::split_module_file(&fixture("emits_resource_and_image_with_tint").join("input.brul"))
            .unwrap();
    assert!(
        root.resources
            .iter()
            .any(|item| item.name == "demo_resource_check" && item.path.is_file()),
        "{:?}",
        root.resources
    );

    let component = emit::split_module_file(
        &fixture("emits_component_resource_with_component_prefix").join("input.brul"),
    )
    .unwrap();
    assert!(
        component
            .resources
            .iter()
            .any(|item| item.name == "badge_resource_check" && item.path.is_file()),
        "{:?}",
        component.resources
    );
}

#[test]
fn resource_without_a_file_path_is_rejected() {
    let source =
        include_str!("fixtures/codegen/resource_without_a_file_path_is_rejected/input.brul");
    let error = emit::module(source).unwrap_err();
    assert!(
        error
            .message()
            .contains("cannot resolve resource './check.png': source has no file path"),
        "{}",
        error.message()
    );
}
