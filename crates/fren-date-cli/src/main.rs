//! `fren` - file renamer that understands dates (CLI binary).
//!
//! Thin consumer of the `fren` library. Parses arguments, builds option
//! structs, calls library functions, formats output. All renaming logic
//! lives in `crates/fren`.

use anstream::println as cprintln;
use anstyle::{AnsiColor, Color, Style};
use clap::{CommandFactory, Parser, Subcommand};
use fren_date::LogSink;
use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use anstream::ColorChoice;

mod output;

/// Style for new (target) FILE names. Bright green.
fn style_new_file() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightGreen)))
}

/// Style for new (target) DIRECTORY names. Bright blue.
/// Distinct from files so the eye picks them out at a glance.
fn style_new_dir() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlue)))
}

/// Style for the unchanged parent path (what comes before the last
/// path component in the old/source path). White: dimmer, recedes
/// visually so the eye focuses on the part being renamed.
fn style_old_parent() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::White)))
}

/// Style for the changed component (the last path component in the old
/// path - the name that's actually being renamed). Bright white: stands
/// out against the dimmer parent path, making the changed part visually
/// dominant.
fn style_old_name() -> Style {
    Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightWhite)))
}

#[derive(Parser, Debug)]
#[command(
    name = "fren",
    version,
    about = "fren - file renamer that understands dates",
    long_about = None,
)]
struct Cli {
    /// Actually perform renames. Without this flag, `fren` only prints
    /// what it would do (dry-run is the default).
    #[arg(long, global = true, default_value_t = false)]
    apply: bool,

    /// Increase verbosity. Stackable: `-v`, `-vv`, `-vvv`.
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Suppress non-error output.
    #[arg(short, long, global = true, default_value_t = false)]
    quiet: bool,

    /// Skip confirmation prompts.
    #[arg(short, long, global = true, default_value_t = false)]
    yes: bool,

    /// Walk subdirectories recursively. Off by default (shallow).
    #[arg(short = 'r', long, global = true, default_value_t = false)]
    recursive: bool,

    /// Color output: auto, always, never.
    #[arg(long, global = true, default_value = "auto", value_parser = ["auto", "always", "never"])]
    color: String,

    /// Skip writing the transaction log.
    #[arg(long, global = true, default_value_t = false)]
    no_log: bool,

    /// Override transaction-log directory.
    #[arg(long, global = true)]
    log_dir: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Rename files and directories with slugify + ISO date detection.
    Rename {
        /// Directories to process.
        #[arg(required = true)]
        directories: Vec<std::path::PathBuf>,

        /// Exclude paths (multi).
        #[arg(short = 'x', long)]
        exclude: Vec<std::path::PathBuf>,

        /// Split CamelCase / PascalCase boundaries with the separator
        /// (e.g. `WhatsApp` -> `Whats-App`). Off by default.
        #[arg(long)]
        split_camel: bool,

        /// Conflict policy when a target already exists: abort or number (default).
        #[arg(long, default_value = "number", value_parser = ["abort", "number"])]
        on_conflict: String,
    },

    /// Merge source directories into a target directory.
    Merge {
        /// Target directory (existing).
        target: std::path::PathBuf,

        /// Source directories (existing). Contents are moved into target.
        #[arg(required = true)]
        sources: Vec<std::path::PathBuf>,
    },

    /// Print shell completions for the given shell.
    Completions {
        /// Shell to generate completions for.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Wire --color before any output so all anstream calls respect it.
    match cli.color.as_str() {
        "always" => ColorChoice::Always.write_global(),
        "never" => ColorChoice::Never.write_global(),
        _ => {} // "auto": anstream default; honors NO_COLOR / CLICOLOR_FORCE
    }

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<(), fren_date::FrenError> {
    match &cli.command {
        Command::Rename {
            directories,
            exclude,
            split_camel,
            on_conflict,
        } => run_rename(&cli, directories, exclude, *split_camel, on_conflict),
        Command::Merge { target, sources } => run_merge(&cli, target, sources),
        Command::Completions { shell } => {
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            clap_complete::generate(*shell, &mut cmd, bin_name, &mut std::io::stdout());
            Ok(())
        }
    }
}

fn run_merge(
    cli: &Cli,
    target: &std::path::Path,
    sources: &[std::path::PathBuf],
) -> Result<(), fren_date::FrenError> {
    let source_refs: Vec<&std::path::Path> =
        sources.iter().map(std::path::PathBuf::as_path).collect();
    let dry_run = !cli.apply;
    let report = fren_date::merge_directories(target, &source_refs, dry_run)?;
    if !cli.quiet {
        if dry_run {
            cprintln!("Would move {} file(s):", report.moved.len());
            for m in &report.moved {
                cprintln!("{}", format_merge_line(&m.to, &m.from));
            }
            if !report.moved.is_empty() {
                cprintln!();
                cprintln!("Re-run with --apply to perform the merge.");
            }
        } else {
            for m in &report.moved {
                cprintln!("{}", format_merge_line(&m.to, &m.from));
            }
            if !report.moved.is_empty() {
                cprintln!();
            }
            let new_s = style_new_file();
            cprintln!(
                "{new_s}Moved {} file(s) into {}{new_s:#}",
                report.moved.len(),
                target.display()
            );
        }
    }
    Ok(())
}

/// Format one merge move:
///
///   `-> <bright>changed-tgt-prefix</bright><dim>common-suffix</dim> from <bright>changed-src-prefix</bright><dim>common-suffix</dim>`
///
/// "Common suffix" = trailing path components identical between source
/// and target. "Changed prefix" = the leading components where they differ
/// (typically the source root vs target root).
fn format_merge_line(to: &std::path::Path, from: &std::path::Path) -> String {
    let to_str = to.to_string_lossy();
    let from_str = from.to_string_lossy();

    let to_parts: Vec<&str> = path_components(&to_str);
    let from_parts: Vec<&str> = path_components(&from_str);

    // Walk from the end, count how many trailing components match.
    let mut common = 0usize;
    let max_common = to_parts.len().min(from_parts.len());
    while common < max_common
        && to_parts[to_parts.len() - 1 - common] == from_parts[from_parts.len() - 1 - common]
    {
        common += 1;
    }

    let to_split = to_parts.len() - common;
    let from_split = from_parts.len() - common;

    let bright = style_new_dir(); // bright blue = changed dir part (matches dir rename color)
    let dim = style_old_name(); // bright white = unchanged common suffix (was white; dim looked weird)

    let to_prefix = join_components(&to_parts[..to_split], to_str.starts_with('/'));
    let to_suffix = join_components(&to_parts[to_split..], false);
    let from_prefix = join_components(&from_parts[..from_split], from_str.starts_with('/'));
    let from_suffix = join_components(&from_parts[from_split..], false);

    // If a side has no "different" prefix (entire path is common), skip
    // the empty bright span so we don't render an empty colored chunk.
    let to_render = if to_prefix.is_empty() {
        format!("{dim}{to_suffix}{dim:#}")
    } else if to_suffix.is_empty() {
        format!("{bright}{to_prefix}{bright:#}")
    } else {
        format!("{bright}{to_prefix}/{bright:#}{dim}{to_suffix}{dim:#}")
    };
    let from_render = if from_prefix.is_empty() {
        format!("{dim}{from_suffix}{dim:#}")
    } else if from_suffix.is_empty() {
        format!("{bright}{from_prefix}{bright:#}")
    } else {
        format!("{bright}{from_prefix}/{bright:#}{dim}{from_suffix}{dim:#}")
    };

    format!("-> {to_render} from {from_render}")
}

/// Split a path into components. Treats `.` as its own component so that
/// `./foo/bar` is `[".", "foo", "bar"]`. Drops empty trailing pieces.
fn path_components(s: &str) -> Vec<&str> {
    s.split('/').filter(|p| !p.is_empty()).collect()
}

/// Join components back. If `leading_slash` is true, prepend `/`.
fn join_components(parts: &[&str], leading_slash: bool) -> String {
    let joined = parts.join("/");
    if leading_slash {
        format!("/{joined}")
    } else {
        joined
    }
}

/// Format one rename plan as a single line:
///
///   `-> NEW_NAME[/] from <bright-white>parent/</bright-white><white>OLD_NAME[/]</white>`
///
/// Directories get a trailing `/`. New name is colored (bright green for
/// files, bright blue for directories). The old path is split into the
/// unchanged parent (bright white) and the changed last component
/// (white) so the eye sees exactly what's being renamed.
fn format_rename_line(plan: &fren_date::RenamePlan) -> String {
    let is_dir = matches!(plan.kind, fren_date::ItemKind::Dir);
    let trailing = if is_dir { "/" } else { "" };

    let new = plan.new_name.to_string_lossy();
    let new_style = if is_dir {
        style_new_dir()
    } else {
        style_new_file()
    };

    let old_name = plan.old_name.to_string_lossy();
    let parent_str = plan.parent.to_string_lossy();

    let parent_style = style_old_parent();
    let name_style = style_old_name();

    // Always end the parent with a `/` for visual clarity.
    let parent_with_slash = if parent_str.ends_with('/') {
        parent_str.into_owned()
    } else {
        format!("{parent_str}/")
    };

    format!(
        "-> {new_style}{new}{trailing}{new_style:#} from \
         {parent_style}{parent_with_slash}{parent_style:#}\
         {name_style}{old_name}{trailing}{name_style:#}"
    )
}

struct CliProgressSink {
    quiet: bool,
}

impl fren_date::ProgressSink for CliProgressSink {
    fn on_rename(&mut self, plan: &fren_date::RenamePlan) {
        if !self.quiet {
            cprintln!("{}", format_rename_line(plan));
        }
    }
}

fn confirm_apply(n: usize) -> io::Result<bool> {
    eprint!("Apply {} rename(s)? [y/N] ", n);
    let _ = io::stderr().flush();
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn run_rename(
    cli: &Cli,
    directories: &[std::path::PathBuf],
    exclude: &[std::path::PathBuf],
    split_camel: bool,
    on_conflict: &str,
) -> Result<(), fren_date::FrenError> {
    let conflict_policy = match on_conflict {
        "abort" => fren_date::ConflictPolicy::Abort,
        "number" => fren_date::ConflictPolicy::Number,
        other => {
            return Err(fren_date::FrenError::NotYetImplemented(format!(
                "unknown --on-conflict value: {other}"
            )));
        }
    };
    let opts = fren_date::RenameOpts {
        slugify: fren_date::SlugOpts {
            split_camel,
            ..fren_date::SlugOpts::default()
        },
        plan: fren_date::PlanOpts {
            recursive: cli.recursive,
            exclude: exclude.to_vec(),
            on_conflict: conflict_policy,
        },
        // `--apply` inverts the default-true `--dry-run`. Other flags
        // (verbose/quiet/color/log) wired in subsequent commits.
        apply: cli.apply,
    };

    let roots: Vec<&std::path::Path> = directories
        .iter()
        .map(std::path::PathBuf::as_path)
        .collect();

    // Dry-run uses high-level fren_date::rename(); apply path uses the explicit
    // plan + execute_with_log so we can write the transaction log.
    let (plans, report) = if !opts.apply {
        fren_date::rename(&roots, &opts)?
    } else {
        let plans = fren_date::plan(&roots, &opts.slugify, &opts.plan)?;

        if plans.is_empty() {
            cprintln!("All names already in canonical form.");
            return Ok(());
        }

        if !cli.yes {
            // Non-TTY stdin without --yes: error rather than hang.
            if !io::stdin().is_terminal() {
                eprintln!("error: --apply requires --yes when stdin is not a terminal");
                return Err(fren_date::FrenError::InvalidInput(
                    "--apply requires --yes when stdin is not a terminal".to_string(),
                ));
            }

            // Build the plan preview as a string and send it through the pager.
            let mut preview = format!("Would rename {} item(s):\n", plans.len());
            for plan in &plans {
                preview.push_str(&format!("{}\n", format_rename_line(plan)));
            }
            output::pager::page(&preview).unwrap_or_else(|_| {
                eprint!("{}", preview);
            });

            if !confirm_apply(plans.len()).map_err(|e| fren_date::FrenError::Io {
                path: std::path::PathBuf::new(),
                source: e,
            })? {
                eprintln!("Aborted.");
                return Ok(());
            }
        }

        let batch_id = plans
            .first()
            .map(|p| p.batch_id)
            .unwrap_or_else(uuid::Uuid::nil);
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let mut progress = CliProgressSink { quiet: cli.quiet };
        let report = if cli.no_log {
            fren_date::execute_with_progress(&plans, &mut fren_date::NullLogSink, &mut progress)?
        } else {
            let mut sink = fren_date::JsonlLogSink::open(cli.log_dir.as_deref(), batch_id, &ts)?;
            sink.append(&fren_date::LogRecord::Batch {
                v: 1,
                id: batch_id,
                ts: chrono::Utc::now().to_rfc3339(),
                cmd: "rename".to_string(),
                args: std::env::args().collect(),
                cwd: std::env::current_dir().unwrap_or_default(),
                fren_version: env!("CARGO_PKG_VERSION").to_string(),
            })?;
            let report = fren_date::execute_with_progress(&plans, &mut sink, &mut progress)?;
            let status = if report.errors.is_empty() {
                "ok"
            } else if report.applied > 0 {
                "partial"
            } else {
                "error"
            };
            sink.append(&fren_date::LogRecord::End {
                v: 1,
                ts: chrono::Utc::now().to_rfc3339(),
                status: status.to_string(),
                applied: report.applied,
                skipped: report.skipped,
                errors: report.errors.len(),
            })?;
            report
        };
        (plans, report)
    };

    if opts.apply {
        if !cli.quiet {
            if report.applied > 0 {
                cprintln!();
            }
            let s = style_new_file();
            cprintln!("{s}Renamed {} item(s).{s:#}", report.applied);
        }
        if !report.errors.is_empty() {
            eprintln!("---");
            eprintln!(
                "error after {} rename(s): {}",
                report.applied, report.errors[0]
            );
        }
    } else if !cli.quiet {
        // Dry-run preview.
        if plans.is_empty() {
            cprintln!("All names already in canonical form.");
        } else {
            cprintln!("Would rename {} item(s):", plans.len());
            for plan in &plans {
                cprintln!("{}", format_rename_line(plan));
            }
            cprintln!();
            cprintln!("Re-run with --apply to perform these renames.");
        }
    }
    Ok(())
}
