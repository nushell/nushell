/// Check at const time the equality of two strings.
pub const fn eq(a: &str, b: &str) -> bool {
    let mut a_bytes = a.as_bytes();
    let mut b_bytes = b.as_bytes();

    if a_bytes.len() != b_bytes.len() {
        return false;
    }

    while let ([a, a_rest @ ..], [b, b_rest @ ..]) = (a_bytes, b_bytes) {
        a_bytes = a_rest;
        b_bytes = b_rest;
        if *a != *b {
            return false;
        }
    }

    true
}
