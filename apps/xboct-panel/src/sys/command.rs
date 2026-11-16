use std::collections::{BTreeMap, BTreeSet};

use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::UI::Shell::CommandLineToArgvW;

use super::text::wide;

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum CommandError {
    #[error("command is empty")]
    Empty,
    #[error("cannot parse command {command:?}")]
    Unparsable { command: String },
}

pub(crate) fn split(command: &str) -> Result<(String, Vec<String>), CommandError> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err(CommandError::Empty);
    }
    let mutated = wide(trimmed);
    let mut argc = 0;
    let argv = unsafe { CommandLineToArgvW(mutated.as_ptr(), &mut argc) };
    if argv.is_null() || argc == 0 {
        return Err(CommandError::Unparsable {
            command: command.to_string(),
        });
    }
    let mut parts = Vec::with_capacity(argc as usize);
    for index in 0..argc as usize {
        let pointer = unsafe { *argv.add(index) };
        let length = unsafe { length_of(pointer) };
        parts.push(String::from_utf16_lossy(unsafe {
            std::slice::from_raw_parts(pointer, length)
        }));
    }
    unsafe {
        LocalFree(argv as *mut core::ffi::c_void);
    }
    let program = parts.remove(0);
    Ok((program, parts))
}

unsafe fn length_of(pointer: *const u16) -> usize {
    let mut length = 0;
    while unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }
    length
}

pub(crate) fn quote(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_string();
    }
    let mut out = String::with_capacity(arg.len() + 2);
    out.push('"');
    let mut backslashes = 0;
    for character in arg.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
                out.push('"');
                backslashes = 0;
            }
            _ => {
                out.extend(std::iter::repeat_n('\\', backslashes));
                out.push(character);
                backslashes = 0;
            }
        }
    }
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
    out
}

pub(crate) fn line(exe: &str, args: &[String]) -> String {
    let mut line = quote(exe);
    for arg in args {
        line.push(' ');
        line.push_str(&quote(arg));
    }
    line
}

pub(crate) fn environment(overrides: &BTreeMap<String, String>) -> Option<Vec<u16>> {
    if overrides.is_empty() {
        return None;
    }
    let replaced: BTreeSet<String> = overrides.keys().map(|key| key.to_uppercase()).collect();
    let mut pairs: Vec<(String, String)> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .filter(|(key, _)| !replaced.contains(&key.to_uppercase()))
        .collect();
    pairs.extend(
        overrides
            .iter()
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    pairs.sort_by_key(|(key, _)| key.to_uppercase());
    let mut block = Vec::new();
    for (key, value) in pairs {
        block.extend(format!("{key}={value}").encode_utf16());
        block.push(0);
    }
    block.push(0);
    Some(block)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quoted_command_with_spaces_is_split() {
        let (program, args) =
            split(r#""C:\Program Files\Deskflow\deskflow-core.exe" server -s "a b.conf""#)
                .expect("command");

        assert_eq!(program, r"C:\Program Files\Deskflow\deskflow-core.exe");
        assert_eq!(
            args,
            vec![
                String::from("server"),
                String::from("-s"),
                String::from("a b.conf")
            ]
        );
    }

    #[test]
    fn a_blank_command_is_rejected() {
        assert!(split("   ").is_err());
    }

    #[test]
    fn a_command_line_round_trips_its_quotes() {
        let joined = line("npm", &[String::from("run"), String::from("dev")]);
        assert_eq!(joined, "npm run dev");
    }

    #[test]
    fn an_environment_block_overrides_and_sorts() {
        let mut overrides = BTreeMap::new();
        overrides.insert(String::from("XBOCT_PANEL_TEST"), String::from("1"));

        let block = environment(&overrides).expect("block");
        let text = String::from_utf16_lossy(&block);

        assert!(text.contains("XBOCT_PANEL_TEST=1\0"), "{text}");
        assert!(text.ends_with("\0\0"), "{text}");
        assert!(environment(&BTreeMap::new()).is_none());
    }
}
