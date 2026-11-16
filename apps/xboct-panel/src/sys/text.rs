pub(crate) fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) fn from_pointer(pointer: *const u16) -> String {
    if pointer.is_null() {
        return String::new();
    }
    let mut length = 0;
    while length < 1024 && unsafe { *pointer.add(length) } != 0 {
        length += 1;
    }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(pointer, length) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_string_is_read_until_the_terminator() {
        let text = wide("Services");
        assert_eq!(from_pointer(text.as_ptr()), "Services");
        assert_eq!(from_pointer(std::ptr::null()), "");
    }
}
