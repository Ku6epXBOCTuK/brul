use std::fmt::Write;

use crate::Resource;

pub(super) fn functions(prefix: &str, resources: &[Resource]) -> String {
    let mut out = String::new();
    for resource in resources {
        let _ = write!(
            out,
            "\npub fn {prefix}resource_{}() -> &'static [u8] {{\n    include_bytes!({:?})\n}}\n",
            resource.name,
            resource.path.to_string_lossy()
        );
    }
    out
}
