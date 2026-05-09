//! Separator manipulation: replace non-alphanumeric characters with the
//! chosen sentinel and collapse runs of consecutive sentinels.

/// Replace every character that is not ASCII alphanumeric and not the
/// `sentinel` itself with `sentinel`.
#[must_use]
pub fn replace_non_alnum(input: &str, sentinel: char) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if c == sentinel || c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            out.push(sentinel);
        }
    }
    out
}

/// Collapse runs of consecutive `sentinel` characters into a single one.
#[must_use]
pub fn collapse_runs(input: &str, sentinel: char) -> String {
    let mut out = String::with_capacity(input.len());
    let mut last_was_sentinel = false;
    for c in input.chars() {
        let is_sentinel = c == sentinel;
        if is_sentinel && last_was_sentinel {
            continue;
        }
        out.push(c);
        last_was_sentinel = is_sentinel;
    }
    out
}
