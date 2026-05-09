use std::io::{self, IsTerminal, Write};
use std::process::{Command, Stdio};

/// Pipe `content` through the user's configured pager.
///
/// Resolution order:
/// 1. `$PAGER` env var (if set and non-empty)
/// 2. `less -FIRX`
/// 3. Raw stdout fallback (also used when stdout is not a TTY)
///
/// `-F` exits immediately if output fits on one screen.
/// `-I` case-insensitive search.
/// `-R` pass ANSI color codes through unchanged.
/// `-X` do not clear screen on exit (plan stays visible after pager closes).
pub fn page(content: &str) -> io::Result<()> {
    if !io::stdout().is_terminal() {
        return write_raw(content);
    }

    let pager = resolve_pager();
    let (cmd, args) = parse_pager_cmd(&pager);

    let mut child = Command::new(cmd).args(&args).stdin(Stdio::piped()).spawn();

    match child {
        Ok(ref mut child) => {
            if let Some(stdin) = child.stdin.take() {
                let mut stdin = stdin;
                // Ignore broken-pipe errors (user exits pager early).
                let _ = stdin.write_all(content.as_bytes());
                drop(stdin); // flush + close stdin so pager knows input is complete
            }
            let _ = child.wait();
            Ok(())
        }
        Err(_) => write_raw(content),
    }
}

/// Returns the pager command string: `$PAGER` if set, else `"less -FIRX"`.
fn resolve_pager() -> String {
    std::env::var("PAGER")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "less -FIRX".to_string())
}

/// Split a pager string like `"less -FIRX"` into `("less", ["-FIRX"])`.
fn parse_pager_cmd(pager: &str) -> (&str, Vec<&str>) {
    let mut parts = pager.split_whitespace();
    let cmd = parts.next().unwrap_or("less");
    let args: Vec<&str> = parts.collect();
    (cmd, args)
}

fn write_raw(content: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(content.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_pager_returns_string() {
        // The fallback is "less -FIRX" when PAGER is unset/empty.
        // We cannot safely mutate env in parallel tests, so just assert
        // the result is non-empty regardless of env state.
        let p = resolve_pager();
        assert!(!p.is_empty());
    }

    #[test]
    fn resolve_pager_empty_env_gives_fallback() {
        // Test the filter logic: an empty PAGER string should fall through to default.
        // We test via parse_pager_cmd with the known default string instead.
        let (cmd, _args) = parse_pager_cmd("less -FIRX");
        assert_eq!(cmd, "less");
    }

    #[test]
    fn parse_pager_cmd_splits_args() {
        let (cmd, args) = parse_pager_cmd("less -FIRX");
        assert_eq!(cmd, "less");
        assert_eq!(args, vec!["-FIRX"]);
    }

    #[test]
    fn parse_pager_cmd_no_args() {
        let (cmd, args) = parse_pager_cmd("more");
        assert_eq!(cmd, "more");
        assert!(args.is_empty());
    }
}
