//! `fren` - file renamer that understands dates (CLI binary).
//!
//! Thin consumer of the `fren` library. Parses arguments, builds option
//! structs, calls library functions, formats output. All renaming logic
//! lives in `crates/fren`.

use anstream::println as cprintln;
use anstyle::{AnsiColor, Color, Style};
use clap::{CommandFactory, Parser, Subcommand};
use fren::LogSink;
use std::process::ExitCode;

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
        /// Directories to process (recursively).
        #[arg(required = true)]
        directories: Vec<std::path::PathBuf>,

        /// Exclude paths (multi).
        #[arg(short = 'x', long)]
        exclude: Vec<std::path::PathBuf>,

        /// Split CamelCase / PascalCase boundaries with the separator
        /// (e.g. `WhatsApp` -> `Whats-App`). Off by default.
        #[arg(long)]
        split_camel: bool,
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

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(cli: Cli) -> Result<(), fren::FrenError> {
    match &cli.command {
        Command::Rename {
            directories,
            exclude,
            split_camel,
        } => run_rename(&cli, directories, exclude, *split_camel),
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
) -> Result<(), fren::FrenError> {
    let source_refs: Vec<&std::path::Path> =
        sources.iter().map(std::path::PathBuf::as_path).collect();
    let dry_run = !cli.apply;
    let report = fren::merge_directories(target, &source_refs, dry_run)?;
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
fn format_rename_line(plan: &fren::RenamePlan) -> String {
    let is_dir = matches!(plan.kind, fren::ItemKind::Dir);
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

fn run_rename(
    cli: &Cli,
    directories: &[std::path::PathBuf],
    exclude: &[std::path::PathBuf],
    split_camel: bool,
) -> Result<(), fren::FrenError> {
    let opts = fren::RenameOpts {
        slugify: fren::SlugOpts {
            split_camel,
            ..fren::SlugOpts::default()
        },
        plan: fren::PlanOpts {
            recursive: true,
            exclude: exclude.to_vec(),
            on_conflict: fren::ConflictPolicy::Abort,
        },
        // `--apply` inverts the default-true `--dry-run`. Other flags
        // (verbose/quiet/color/log) wired in subsequent commits.
        apply: cli.apply,
    };

    let roots: Vec<&std::path::Path> = directories
        .iter()
        .map(std::path::PathBuf::as_path)
        .collect();

    // Dry-run uses high-level fren::rename(); apply path uses the explicit
    // plan + execute_with_log so we can write the transaction log.
    let (plans, report) = if !opts.apply {
        fren::rename(&roots, &opts)?
    } else {
        let plans = fren::plan(&roots, &opts.slugify, &opts.plan)?;
        let batch_id = plans
            .first()
            .map(|p| p.batch_id)
            .unwrap_or_else(uuid::Uuid::nil);
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let report = if cli.no_log || plans.is_empty() {
            fren::execute(&plans)?
        } else {
            let mut sink = fren::JsonlLogSink::open(cli.log_dir.as_deref(), batch_id, &ts)?;
            // Header
            sink.append(&fren::LogRecord::Batch {
                v: 1,
                id: batch_id,
                ts: chrono::Utc::now().to_rfc3339(),
                cmd: "rename".to_string(),
                args: std::env::args().collect(),
                cwd: std::env::current_dir().unwrap_or_default(),
                fren_version: env!("CARGO_PKG_VERSION").to_string(),
            })?;
            let report = fren::execute_with_log(&plans, &mut sink)?;
            // End marker
            let status = if report.errors.is_empty() {
                "ok"
            } else if report.applied > 0 {
                "partial"
            } else {
                "error"
            };
            sink.append(&fren::LogRecord::End {
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
        // Abort policy: the first `report.applied` plans succeeded (in
        // bottom-up order), the next one (if any) failed. Print the applied
        // ones using the same format as dry-run so the user can verify
        // what changed.
        for plan in plans.iter().take(report.applied) {
            cprintln!("{}", format_rename_line(plan));
        }
        if report.applied > 0 {
            cprintln!();
        }
        let s = style_new_file();
        cprintln!("{s}Renamed {} item(s).{s:#}", report.applied);
        if !report.errors.is_empty() {
            for e in &report.errors {
                eprintln!("error: {e}");
            }
        }
    } else {
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
