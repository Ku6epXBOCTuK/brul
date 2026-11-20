use std::fmt::Write;

use crate::Resource;

pub(super) fn function_name(prefix: &str, name: &str) -> String {
    format!("{prefix}resource_{name}")
}

pub(super) fn functions(prefix: &str, resources: &[Resource]) -> String {
    let mut out = String::new();
    for resource in resources {
        let _ = write!(
            out,
            "\npub fn {}() -> &'static [u8] {{\n    include_bytes!(concat!(env!(\"OUT_DIR\"), \"/resources/{}\"))\n}}\n",
            function_name(prefix, &resource.name),
            function_name(prefix, &resource.name),
        );
    }
    out
}
