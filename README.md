# fren

**File renamer that understands dates - with slugify, Unicode, and CamelCase smarts**

`fren` is a command-line tool for batch-renaming files and directories. It detects dates inside filenames in 17+ formats, converts them to ISO 8601, slugifies the rest of the name, splits CamelCase, normalizes Unicode, and lowercases extensions. It also includes a separate `merge` command for combining directories with automatic conflict resolution.

> **Breaking change in 0.3.0:** `fren rename` now processes only the immediate
> directory by default. Pass `-r` / `--recursive` to walk subdirectories as before.

## Features

- 📅 **Date Detection**: Recognizes 17+ formats inside filenames (human-readable, ISO, datetime, minute-precision) and rewrites them as ISO 8601
- 🔤 **Slugification**: Cleans filenames by replacing spaces and punctuation with a single separator (default `-`)
- 🐫 **CamelCase Splitting**: `WhatsApp` becomes `Whats-App`, `JSONFile` becomes `JSON-File`
- 🌍 **Unicode Normalization**: Strips accents (`Bancários` -> `Bancarios`) via NFKC + ASCII transliteration
- 📁 **Directory Merging**: A separate `fren merge` command moves files between directories, appending `_Copy` / `_Copy1` / `_Copy2` on conflicts
- 🔍 **Exclusions**: Skip specific paths with `-x`
- 🙈 **Hidden Files**: Entries starting with `.` are skipped automatically
- 🔒 **Dry-run by default**: `fren rename DIR` previews; `--apply` is required to actually rename
- 🎨 **Colored output**: Files in bright green, directories in bright blue, with the unchanged parent path dimmed
- 📜 **Transaction log**: Every applied batch is recorded as JSONL under `${XDG_STATE_HOME:-~/.local/state}/fren/log/`

## Installation

From source:

```bash
git clone https://github.com/andreoliwa/fren.git
cd fren
cargo install --path crates/fren-cli
```

This installs the `fren` binary to `~/.cargo/bin/`.

## Commands

### `fren rename`

Rename files and directories with slugify + ISO date detection.

```bash
fren rename [OPTIONS] DIRECTORIES...
```

Options:

- `-x, --exclude PATH`: Exclude one or more paths (can be repeated)
- `--apply`: Actually perform the renames (without this, `fren` only prints what it would do)
- `--no-log`: Skip writing the transaction log
- `--log-dir DIR`: Override transaction-log directory

Examples:

```bash
# Preview (dry-run is the default)
fren rename ~/Documents/MyFiles

# Actually rename
fren rename --apply ~/Documents/MyFiles

# Multiple directories with exclusions
fren rename --apply -x ~/temp/skip -x ~/temp/important.txt ~/temp
```

### `fren reorder`

Move detected dates to the front of each filename, placing them before any other name components.

```bash
fren reorder [OPTIONS] PATHS...
```

Options:

- `-x, --exclude PATH`: Exclude one or more paths (can be repeated)
- `-r, --recursive`: Walk subdirectories recursively
- `--apply`: Actually perform the renames (without this, prints a dry-run preview)
- `--on-conflict abort|number`: Conflict policy when a target already exists (default: `number`)

Examples:

```bash
# Preview
fren reorder ~/Documents/Photos

# Apply
fren reorder --apply ~/Documents/Photos

# Recursive
fren reorder --apply -r ~/Documents
```

### `fren merge`

Merge source directories into a target directory. Move-only - filenames are preserved (with `_Copy` suffixes on conflicts). If you also want to rename the merged contents, run `fren rename` afterwards.

```bash
fren merge [OPTIONS] TARGET SOURCES...
```

Options:

- `--apply`: Actually perform the moves

Examples:

```bash
# Preview
fren merge ~/Documents/Target ~/Documents/Source1 ~/Documents/Source2

# Apply
fren merge --apply ~/Documents/Target ~/Documents/Source1 ~/Documents/Source2

# Merge several into the current directory
fren merge --apply . src1/ src2/ src3/
```

### `fren completions`

Print shell completions.

```bash
fren completions bash > ~/.local/share/bash-completion/completions/fren
fren completions zsh  > ~/.zsh/completions/_fren
fren completions fish > ~/.config/fish/completions/fren.fish
```

## How rename works

The pipeline transforms filenames in this order:

1. **Unicode normalize** (NFKC) and transliterate non-ASCII to ASCII
2. **Inject separator at CamelCase boundaries** (`WhatsApp` -> `Whats_App`)
3. **Inject separator at "at"-time patterns** (`2019-08-21 at 14.24.19` -> `2019-08-21_14_24_19`)
4. **Slugify**: replace whitespace and punctuation with the internal separator
5. **Detect dates** and rewrite them as ISO 8601
6. **Apply case mode** (default: preserve original case)
7. **Collapse consecutive separators** and **substitute to user separator** (default `-`)
8. **Lowercase the file extension**

### Examples

```text
Hello World 2024-01-15.txt           -> Hello-World-2024-01-15.txt
WhatsApp Image 2024-01-15 at 12.30.45.jpg -> Whats-App-Image-2024-01-15T12-30-45.jpg
CamelCaseFile.PDF                    -> Camel-Case-File.pdf
Bancários.txt                        -> Bancarios.txt
report-25-04-2017.pdf                -> report-2017-04-25.pdf
photo_20191020.jpg                   -> photo-2019-10-20.jpg
2026-05-03-18-57.log                 -> 2026-05-03T18-57-00.log
```

### Supported date formats

- **Human-readable**: `DD_MM_YYYY`, `DD/MM/YYYY`, `DD.MM.YYYY`, `DD-MM-YY`, `DDMMYYYY`, `DDMMYY`, `MM_YYYY`
- **ISO / inverted**: `YYYY-MM-DD`, `YYYY_MM_DD`, `YYYYMMDD`, `YYYY_MM`
- **Datetime (full)**: `DD_MM_YYYY_HH_mm_ss`, `YYYY_MM_DD_HH_mm_ss`, `YYYYMMDDHHmmss`, `YYYYMMDD_HHmmss`, `DD_MM_YY_HH_mm_ss`, `YY_MM_DD_HH_mm_ss`
- **Datetime (minute-precision, zero-second pad)**: `DD_MM_YYYY_HH_mm`, `YYYY_MM_DD_HH_mm`, `DDMMYYYYHHmm`

Two-digit years between `(current_year + 10)` and `99` are interpreted as 19YY; otherwise 20YY. With the system clock at 2026 this means `30..=99` -> 1930..1999 and `00..=29` -> 2000..2029, except dates more than 10 years in the future, which roll back a century.

## How merge works

`fren merge TARGET SOURCES...`:

1. Walks each source recursively
2. Computes the target path = `TARGET / relative_subpath_from_source`
3. If the target file already exists (or another move in the batch already claimed that path), appends `_Copy`, `_Copy1`, `_Copy2`, ... to the stem until a free name is found
4. Creates intermediate directories as needed
5. Moves the file with `std::fs::rename`
6. Skips `.DS_Store` and similar metadata files

Source directory structure is preserved verbatim. Only files are moved; empty source directories remain.

## Architecture

`fren` is a Cargo workspace with three crates:

- **`crates/slug-preserve`** (internal): a case-preserving slugifier. Unlike most Rust slug crates that always lowercase, this one supports five case modes: `Preserve`, `Lower`, `Upper`, `Title`, `Capitalize`.
- **`crates/fren`**: the library. Exposes `slugify_camel_iso`, `plan`, `execute`, `merge_directories`, `unique_file_name`, plus the public type surface (`RenamePlan`, `DetectedDate`, `FrenError`, `ConflictPolicy`, `SlugOpts`, `LogSink`, `JsonlLogSink`, etc.). No CLI dependencies; library discipline lints deny `print_stdout`, `print_stderr`, `panic`, `unwrap_used`, `expect_used`.
- **`crates/fren-cli`**: a thin clap-derive binary that consumes the library.

The library is the source of truth. The binary parses arguments, builds option structs, and formats output. Other Rust projects can embed `fren` directly without spawning a subprocess.

## Safety

- **Dry-run is the default**. `--apply` is required to mutate the filesystem.
- **No silent overwrites**. Every rename pre-checks the target and refuses to proceed if it exists outside the batch (the `Abort` conflict policy).
- **Within-batch collisions are detected at planning time**. If two source paths would rename to the same target, the batch aborts before any I/O happens.
- **Bottom-up execution**. Deeper paths are renamed first, so a directory rename never invalidates the queued paths of its children. This fixes a class of bugs that affected the original Python implementation.
- **Case-only renames** on case-insensitive filesystems (macOS APFS, Windows NTFS) route through a temporary name to avoid silent no-ops.
- **Transaction log**. Every applied batch writes a JSONL file under `${XDG_STATE_HOME:-~/.local/state}/fren/log/<timestamp>-<batch-uuid>.jsonl` so applied changes can be audited or, in the future, undone.

## Development

```bash
cargo build           # debug build
cargo test            # 48 tests across slugify, planner, executor, merge
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo install --path crates/fren-cli   # install/refresh ~/.cargo/bin/fren
```

Pre-commit:

```bash
prek install
prek run
```

## License

See the project repository for license information.

## Author

W. Augusto Andreoli (andreoliwa@sent.com)
