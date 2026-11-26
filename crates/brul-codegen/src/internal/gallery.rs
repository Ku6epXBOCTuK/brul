use std::fmt::Write as _;

use brul_syntax::markup::{Arg, Block, BlockItem, Member, Prop, PropType};

use crate::kit;

struct Section {
    title: String,
    args: String,
}

struct Component {
    file: String,
    name: String,
    rank: u8,
    children: bool,
    sections: Vec<Section>,
}

pub fn source() -> Result<String, brul_syntax::error::Error> {
    let mut components = Vec::new();
    for file in kit::names() {
        let text = kit::source(file).expect("the kit table has a source for every name");
        let parsed = crate::parse_source(text)?;
        let declaration = parsed
            .declaration
            .expect("a parsed source always has a declaration");
        let name = declaration.name.clone();
        let prefix = name.to_ascii_lowercase();
        let mut rank = 0;
        let mut modal = false;
        let mut children = false;
        for member in &declaration.members {
            let Member::Element(element) = member else {
                continue;
            };
            children |= has_default_slot(&element.block);
            if element.name == "Layer" {
                rank = 1;
                modal |= element.args.iter().any(|arg| {
                    matches!(
                        arg,
                        Arg::Named { name, value }
                            if name == "modal" && matches!(&value.kind, brul_syntax::ast::ExprKind::Bool(true))
                    )
                });
            }
        }
        if modal {
            rank = 2;
        }
        let props: Vec<&Prop> = declaration
            .members
            .iter()
            .filter_map(|member| match member {
                Member::Prop(prop) => Some(prop),
                _ => None,
            })
            .collect();
        let classes: Vec<String> = parsed
            .stylesheet
            .rules
            .iter()
            .filter(|rule| rule.state.is_none() && !rule.class.starts_with(&prefix))
            .map(|rule| rule.class.clone())
            .collect();
        let mut sections = vec![Section {
            title: "default".to_owned(),
            args: render(&props, &[], &name),
        }];
        for prop in &props {
            if prop.ty == PropType::Bool && !(modal && prop.name == "open") {
                let value = if literal(prop).as_deref() == Some("true") {
                    "false"
                } else {
                    "true"
                };
                sections.push(Section {
                    title: format!("{}: {}", prop.name, value),
                    args: render(&props, &[(prop.name.as_str(), value.to_owned())], &name),
                });
            }
            if prop.name == "variant" {
                for class in &classes {
                    if literal(prop).as_deref() == Some(quote(class).as_str()) {
                        continue;
                    }
                    sections.push(Section {
                        title: format!("variant: {class}"),
                        args: render(&props, &[("variant", quote(class))], &name),
                    });
                }
            }
        }
        components.push(Component {
            file: file.to_owned(),
            name,
            rank,
            children,
            sections,
        });
    }
    components.sort_by(|left, right| (left.rank, &left.file).cmp(&(right.rank, &right.file)));
    Ok(window(&components))
}

fn has_default_slot(block: &Block) -> bool {
    block.items.iter().any(|item| match item {
        BlockItem::Slot(slot) => {
            slot.name.is_none() || slot.block.as_ref().is_some_and(has_default_slot)
        }
        BlockItem::Element(element) => has_default_slot(&element.block),
        BlockItem::If(block) => {
            block
                .branches
                .iter()
                .any(|branch| has_default_slot(&branch.block))
                || block.else_block.as_ref().is_some_and(has_default_slot)
        }
        BlockItem::Each(block) => {
            has_default_slot(&block.block)
                || block.else_block.as_ref().is_some_and(has_default_slot)
        }
        BlockItem::Expr(_) => false,
    })
}

fn render(props: &[&Prop], overrides: &[(&str, String)], fallback: &str) -> String {
    props
        .iter()
        .filter_map(|prop| {
            let value = overrides
                .iter()
                .find(|(name, _)| *name == prop.name)
                .map(|(_, value)| value.clone())
                .or_else(|| literal(prop))
                .or_else(|| match prop.ty {
                    PropType::String => Some(quote(fallback)),
                    PropType::Bool => Some("false".to_owned()),
                    PropType::Number => Some("0".to_owned()),
                    PropType::Null => Some("null".to_owned()),
                    _ => None,
                })?;
            Some(format!("{}: {}", prop.name, value))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn literal(prop: &Prop) -> Option<String> {
    use brul_syntax::ast::ExprKind;

    match &prop.default.as_ref()?.kind {
        ExprKind::String(value) => (!value.is_empty()).then(|| quote(value)),
        ExprKind::Bool(value) => Some(value.to_string()),
        ExprKind::Number(value) => Some(value.to_string()),
        ExprKind::Null => Some("null".to_owned()),
        ExprKind::Ident(name) => Some(name.clone()),
        _ => None,
    }
}

fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn line(out: &mut String, text: impl std::fmt::Display) {
    writeln!(out, "{text}").expect("writing to a String cannot fail");
}

fn window(components: &[Component]) -> String {
    let mut out = String::new();
    line(&mut out, "<script>");
    line(&mut out, "    function toggleTheme(): Void {");
    line(
        &mut out,
        "        setTheme(theme() == \"dark\" ? \"light\" : \"dark\");",
    );
    line(&mut out, "    }");
    line(&mut out, "");
    line(&mut out, "    function handleKey(ev: Object): Void {");
    line(&mut out, "        if (ev.code == \"Escape\") { exit(); }");
    line(
        &mut out,
        "        if (ev.code == \"T\" || ev.code == \"t\") { toggleTheme(); }",
    );
    line(&mut out, "    }");
    line(&mut out, "</script>");
    line(&mut out, "");
    line(&mut out, "<use>");
    for component in components {
        line(
            &mut out,
            format!(
                "    import {} from \"brul:{}\";",
                component.name, component.file
            ),
        );
    }
    line(&mut out, "</use>");
    line(&mut out, "");
    line(&mut out, "<markup>");
    line(&mut out, "    window KitGallery (");
    line(&mut out, "        title: \"kit gallery\",");
    line(&mut out, "        width: 900,");
    line(&mut out, "        height: 720,");
    line(&mut out, "        centered: true,");
    line(&mut out, "        on key: handleKey");
    line(&mut out, "    ) {");
    line(&mut out, "        Container (root) {");
    line(&mut out, "            Text (heading) { \"BRUL kit\" }");
    line(
        &mut out,
        "            Text (hint) { \"T switches theme, Esc exits\" }",
    );
    if components
        .iter()
        .any(|component| component.name == "Button")
    {
        line(
            &mut out,
            "            Button (theme-toggle, label: \"Switch theme\", on clicked: toggleTheme) { }",
        );
    }
    for component in components {
        line(
            &mut out,
            format!("            Container (section-{}) {{", component.file),
        );
        line(
            &mut out,
            format!(
                "                Text (section-title) {{ \"{}\" }}",
                component.name
            ),
        );
        for section in &component.sections {
            line(&mut out, "                Container (state-row) {");
            line(
                &mut out,
                format!(
                    "                    Text (state-label) {{ \"{}\" }}",
                    section.title
                ),
            );
            let open = if section.args.is_empty() {
                format!("{} {{", component.name)
            } else {
                format!("{} ({}) {{", component.name, section.args)
            };
            let children = if component.children {
                format!("Text (state-sample) {{ \"{}\" }} }}", component.name)
            } else {
                "}".to_owned()
            };
            line(&mut out, format!("                    {open} {children}"));
            line(&mut out, "                }");
        }
        line(&mut out, "            }");
    }
    line(&mut out, "        }");
    line(&mut out, "    }");
    line(&mut out, "</markup>");
    line(&mut out, "");
    line(&mut out, "<style>");
    for theme in [
        "dark: #12141c, #1c2028, #232936, #334155, #ffffff, #8c94a8, #4c8dff, #6ba0ff",
        "light: #f4f6f9, #ffffff, #eef2f7, #e2e8f0, #0f172a, #64748b, #2563eb, #1d4ed8",
    ] {
        let (name, colors) = theme.split_once(": ").expect("a theme has a palette");
        let colors: Vec<&str> = colors.split(", ").collect();
        line(&mut out, format!("    theme {name} {{"));
        for (key, value) in [
            "background",
            "surface",
            "surface-hover",
            "border",
            "text",
            "muted",
            "accent",
            "accent-hover",
        ]
        .iter()
        .zip(colors)
        {
            line(&mut out, format!("        {key}: {value};"));
        }
        line(&mut out, "    }");
    }
    line(
        &mut out,
        "    .root { width: 100%; height: 100%; padding: 18px; gap: 12px; background: theme.background; }",
    );
    line(
        &mut out,
        "    .heading { color: theme.text; font-size: 20px; font-weight: 600; }",
    );
    line(
        &mut out,
        "    .hint { color: theme.muted; font-size: 12px; }",
    );
    line(
        &mut out,
        "    .section { width: 100%; padding: 12px; gap: 8px; background: theme.surface; border: 1px theme.border; border-radius: 12px; }",
    );
    line(
        &mut out,
        "    .section-title { color: theme.text; font-size: 15px; font-weight: 600; }",
    );
    line(
        &mut out,
        "    .state-row { width: 100%; gap: 10px; align-items: center; }",
    );
    line(
        &mut out,
        "    .state-label { width: 130px; color: theme.muted; font-size: 11px; }",
    );
    line(
        &mut out,
        "    .state-sample { color: theme.text; font-size: 13px; }",
    );
    line(&mut out, "</style>");
    out
}

#[cfg(test)]
mod tests {
    use super::source;

    #[test]
    fn every_kit_component_has_a_section() {
        let source = source().expect("the kit gallery source is generated");
        for name in crate::kit::names() {
            assert!(
                source.contains(&format!("section-{name}")),
                "no gallery section for {name}"
            );
        }
        assert!(source.contains("variant: primary"));
        assert!(source.contains("disabled: true"));
    }
}
