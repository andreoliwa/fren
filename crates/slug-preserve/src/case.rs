//! Case-mode handling.

/// How to handle character case in the slugified output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseMode {
    /// Keep the original case of each character.
    Preserve,
    /// Lowercase the entire string.
    Lower,
    /// Uppercase the entire string.
    Upper,
    /// Title-case: first char of each word uppercase, rest lowercase. Word
    /// boundaries are non-alphanumeric chars and start of string. This
    /// matches the *effective* output of the Python `slugify_camel_iso`
    /// pipeline (`.capitalize()` followed by `_[a-z] → _X`).
    Title,
    /// Synonym for [`CaseMode::Title`] kept for clarity in CLI flag values
    /// (`--case=capitalize`).
    Capitalize,
}

/// Apply a case mode to a string.
#[must_use]
pub fn apply(input: &str, mode: CaseMode) -> String {
    match mode {
        CaseMode::Preserve => input.to_string(),
        CaseMode::Lower => input.to_lowercase(),
        CaseMode::Upper => input.to_uppercase(),
        CaseMode::Title | CaseMode::Capitalize => title_case(input),
    }
}

fn title_case(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut at_word_start = true;
    for c in input.chars() {
        if !c.is_ascii_alphanumeric() {
            out.push(c);
            at_word_start = true;
            continue;
        }
        if at_word_start {
            for u in c.to_uppercase() {
                out.push(u);
            }
            at_word_start = false;
        } else {
            for l in c.to_lowercase() {
                out.push(l);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserve_unchanged() {
        assert_eq!(apply("Hello-World", CaseMode::Preserve), "Hello-World");
    }

    #[test]
    fn lower_lowercases() {
        assert_eq!(apply("Hello-World", CaseMode::Lower), "hello-world");
    }

    #[test]
    fn upper_uppercases() {
        assert_eq!(apply("Hello-World", CaseMode::Upper), "HELLO-WORLD");
    }

    #[test]
    fn title_capitalizes_each_word() {
        assert_eq!(apply("hello-world", CaseMode::Title), "Hello-World");
    }

    #[test]
    fn capitalize_same_as_title() {
        assert_eq!(
            apply("hello-world", CaseMode::Capitalize),
            apply("hello-world", CaseMode::Title)
        );
    }

    #[test]
    fn title_case_lowercases_non_first_chars() {
        assert_eq!(apply("HELLO WORLD", CaseMode::Title), "Hello World");
    }
}
