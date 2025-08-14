use windows::core::PCWSTR;

pub fn wide_null(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}

pub fn pcw(s: &str) -> PCWSTR {
    PCWSTR(wide_null(s).as_ptr())
}

