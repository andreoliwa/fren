# fren

**File renamer with slugify and date detection**

`fren` is a powerful command-line tool for batch renaming files and directories with intelligent slugification
and automatic date formatting. It also includes a utility for merging directories while handling file conflicts.

## Features

- 🔤 **Smart Slugification**: Converts file and directory names to clean, URL-friendly formats
- 📅 **Intelligent Date Detection**: Automatically detects and reformats dates to ISO 8601 format
- 🐫 **CamelCase Handling**: Properly splits and formats CamelCase and PascalCase names
- 🌍 **Unicode Support**: Handles accented characters and special symbols
- 📁 **Directory Merging**: Merge multiple directories with automatic conflict resolution
- 🔍 **Selective Processing**: Exclude specific files and directories from operations
- 🔒 **Safe Operations**: Dry-run mode to preview changes before applying them
- 🙈 **Hidden File Handling**: Automatically ignores hidden files and directories

## Installation

```bash
pip install fren
```

Or install from source:

```bash
git clone <repository-url>
cd fren
pip install -e .
```

## Scripts

### 1. `rename-slugify` (Primary Tool)

The main script for intelligently renaming files and directories with slugification and date formatting.

#### Usage

```bash
rename-slugify [OPTIONS] DIRECTORIES...
```

#### Options

- `-x, --exclude PATH`: Exclude one or more directories or files from processing (can be used multiple times)
- `-y, --yes`: Answer yes to all prompts (non-interactive mode)
- `-n, --dry-run`: Show what would be done without actually renaming anything
- `-v, --verbose`: Display detailed information about the renaming process

#### Examples

**Basic usage - rename all files in a directory:**

```bash
rename-slugify ~/Documents/MyFiles
```

**Dry-run to preview changes:**

```bash
rename-slugify --dry-run ~/Documents/MyFiles
```

**Exclude specific directories:**

```bash
rename-slugify --exclude ~/Documents/MyFiles/KeepOriginal ~/Documents/MyFiles
```

**Process multiple directories with auto-confirm:**

```bash
rename-slugify --yes ~/Documents/Folder1 ~/Documents/Folder2
```

**Verbose mode with exclusions:**

```bash
rename-slugify -v -x ~/temp/skip -x ~/temp/important.txt ~/temp
```

#### How It Works

The `rename-slugify` script transforms file and directory names using these rules:

1. **Slugification**: Converts names to lowercase with underscores, then capitalizes each word
   - `"My Document.pdf"` → `"My_Document.pdf"`
   - `"some-file here.txt"` → `"Some_File_Here.txt"`

2. **CamelCase Splitting**: Intelligently splits CamelCase and PascalCase
   - `"CamelCaseFile.doc"` → `"Camel_Case_File.doc"`
   - `"JSONParser.py"` → `"Jsonparser.py"`
   - `"WhatsApp Image.jpg"` → `"Whats_App_Image.jpg"`

3. **Date Formatting**: Detects various date formats and converts to ISO 8601
   - `"report-25-04-2017.pdf"` → `"Report_2017-04-25.pdf"`
   - `"photo_20191020.jpg"` → `"Photo_2019-10-20.jpg"`
   - `"scan 30.12.2017.pdf"` → `"Scan_2017-12-30.pdf"`
   - `"backup_240819.zip"` → `"Backup_2019-08-24.zip"`

4. **DateTime Formatting**: Handles timestamps with time components
   - `"log_2017_12_30_10_44_56.txt"` → `"Log_2017-12-30T10-44-56.txt"`
   - `"WhatsApp Ptt 2019-08-21 at 14.24.19.opus"` → `"Whats_App_Ptt_2019-08-21T14-24-19.opus"`
   - `"video 20180726_224001.mp4"` → `"Video_2018-07-26T22-40-01.mp4"`

5. **Unicode Normalization**: Handles accented characters
   - `"Atenção.txt"` → `"Atencao.txt"`
   - `"Bancários.pdf"` → `"Bancarios.pdf"`

6. **File Extension**: Converts extensions to lowercase
   - `"Document.PDF"` → `"Document.pdf"`

#### Supported Date Formats

The script recognizes and converts these date formats:

- Human-readable: `DD/MM/YYYY`, `DD.MM.YYYY`, `DD_MM_YYYY`, `DD-MM-YY`
- ISO-like: `YYYY-MM-DD`, `YYYY_MM_DD`, `YYYYMMDD`
- With time: `DD_MM_YYYY_HH_mm_ss`, `YYYY-MM-DDTHH-mm-ss`, `YYYYMMDDHHmmss`
- Month-only: `MM/YYYY`, `YYYY-MM`

The script intelligently handles 2-digit years, treating them as between 1929 and 2029.

#### Special Features

- **Hidden Files**: Automatically skips files and directories starting with `.`
- **Directory Merging**: If renaming a directory would conflict with an existing directory, automatically merges them
- **Batch Processing**: Processes directories first, then files, ensuring proper hierarchy
- **Relative Paths**: Displays paths relative to home directory (`~`) for cleaner output

### 2. `merge-dirs`

Merge multiple source directories into a single target directory, handling file conflicts automatically.

#### Usage

```bash
merge-dirs [OPTIONS] TARGET_DIRECTORY SOURCE_DIRECTORIES...
```

#### Options

- `-n, --dry-run`: Show what would be done without actually moving files

#### Examples

**Merge two directories into one:**

```bash
merge-dirs ~/Documents/Target ~/Documents/Source1 ~/Documents/Source2
```

**Preview merge operation:**

```bash
merge-dirs --dry-run ~/Documents/Target ~/Documents/Source1
```

#### How It Works

1. **Preserves Structure**: Maintains the directory structure from source directories
2. **Conflict Resolution**: If a file with the same name exists in the target:
   - Appends `_Copy` to the filename
   - If `_Copy` exists, appends `_Copy1`, `_Copy2`, etc.
3. **Ignores System Files**: Automatically skips `.DS_Store` and similar files
4. **Creates Directories**: Automatically creates necessary subdirectories in the target

#### Example Conflict Resolution

```
Target/document.pdf (exists)
Source/document.pdf → Target/document_Copy.pdf
Source2/document.pdf → Target/document_Copy1.pdf
```

## Requirements

- Python >= 3.13
- Dependencies:
  - `click`: Command-line interface
  - `pendulum`: Date/time parsing and formatting
  - `python-slugify`: Text slugification

## Development

### Running Tests

```bash
pytest
```

### Code Quality

The project uses Ruff for linting and formatting:

```bash
ruff check src/
ruff format src/
```

## Use Cases

- **Photo Organization**: Rename photos with dates from various cameras and phones
- **Document Management**: Clean up document names from different sources
- **Archive Cleanup**: Standardize file names in old archives
- **WhatsApp Media**: Format WhatsApp image/video/audio file names properly
- **Project Files**: Organize project files with consistent naming
- **Directory Consolidation**: Merge duplicate or related directories

## Tips

1. **Always use `--dry-run` first** to preview changes before applying them
2. **Use `--verbose`** to understand what's being skipped or excluded
3. **Exclude important directories** with `-x` to avoid accidental renames
4. **Process in batches** rather than your entire file system at once
5. **Backup important data** before running batch operations

## License

See the project repository for license information.

## Author

W. Augusto Andreoli (andreoliwa@sent.com)
