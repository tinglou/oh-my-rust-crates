//! `erfs-tool` inspects ERFS (Embedded Resource Filesystem) images.
//!
//! It provides the same three commands as the C reference tool in
//! `erfs-c/src/tool/erfs_tool.c`:
//!
//! ```text
//! erfs-tool ls <dir> <img>             - list directory entries
//! erfs-tool cat [-c | -x] <path> <img> - print file content
//! erfs-tool tree [-d] <dir> <img>      - print directory tree
//! ```

use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use comfy_table::{Cell, CellAlignment, Table, presets::NOTHING};
use erfs_rt::{Entry, Fs, OwnedFs};

/// Number of bytes displayed per line by the dump modes.
const DUMP_BYTES_PER_LINE: usize = 16;

/// Inspect ERFS (Embedded Resource Filesystem) images.
#[derive(Debug, Parser)]
#[command(name = "erfs-tool", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List directory entries
    Ls {
        /// Directory path inside the image
        #[arg(value_name = "DIR")]
        path: String,
        /// ERFS image file
        #[arg(value_name = "IMG")]
        image: PathBuf,
    },
    /// Print file content
    Cat {
        /// C-style dump: 16 bytes per line with escapes and hex
        #[arg(short = 'c', conflicts_with = "hex")]
        c_style: bool,
        /// Hexadecimal dump: 16 bytes per line
        #[arg(short = 'x', long = "hex", conflicts_with = "c_style")]
        hex: bool,
        /// File path inside the image
        #[arg(value_name = "PATH")]
        path: String,
        /// ERFS image file
        #[arg(value_name = "IMG")]
        image: PathBuf,
    },
    /// Print directory tree
    Tree {
        /// Only show directories, ignoring files
        #[arg(short = 'd', long = "dirs-only")]
        dirs_only: bool,
        /// Directory path inside the image
        #[arg(value_name = "DIR")]
        path: String,
        /// ERFS image file
        #[arg(value_name = "IMG")]
        image: PathBuf,
    },
}

impl Command {
    fn path(&self) -> &str {
        match self {
            Command::Ls { path, .. } | Command::Cat { path, .. } | Command::Tree { path, .. } => {
                path
            }
        }
    }

    fn image(&self) -> &Path {
        match self {
            Command::Ls { image, .. }
            | Command::Cat { image, .. }
            | Command::Tree { image, .. } => image,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let image_path = cli.command.image();
    let path = cli.command.path();

    let bytes = fs::read(image_path)
        .map_err(|err| format!("cannot open file '{}': {err}", image_path.display()))?;
    let owned = OwnedFs::from_bytes(bytes)
        .map_err(|err| format!("invalid ERFS image '{}': {err}", image_path.display()))?;
    let fs = owned.as_fs();

    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    match &cli.command {
        Command::Ls { .. } => cmd_ls(&fs, path, &mut out)?,
        Command::Cat { c_style, hex, .. } => {
            let mode = if *c_style {
                CatMode::CStyle
            } else if *hex {
                CatMode::Hex
            } else {
                CatMode::Raw
            };
            cmd_cat(&fs, path, mode, &mut out)?;
        }
        Command::Tree { dirs_only, .. } => cmd_tree(&fs, path, *dirs_only, &mut out)?,
    }

    out.flush()
        .map_err(|err| format!("failed to write output: {err}"))?;
    Ok(())
}

/// Lists the direct children of `path`.
fn cmd_ls<W: Write>(fs: &Fs<'_>, path: &str, out: &mut W) -> Result<(), String> {
    let dir = fs
        .open(path)
        .map_err(|err| format!("cannot open '{path}': {err}"))?;
    if !dir.is_dir() {
        return Err(format!("'{path}' is not a directory"));
    }

    let rows = dir
        .children()
        .map_err(|err| format!("cannot read directory '{path}': {err}"))?
        .map(|entry| Row::new(entry.name(), &entry))
        .collect();
    render_table(out, rows)
}

/// Output mode of the `cat` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CatMode {
    /// Write the file bytes unchanged.
    Raw,
    /// Escape non-printable bytes and show them 16 per line.
    CStyle,
    /// Show the file as a hexadecimal dump.
    Hex,
}

/// Writes the content of a file to `out`.
fn cmd_cat<W: Write>(fs: &Fs<'_>, path: &str, mode: CatMode, out: &mut W) -> Result<(), String> {
    let entry = fs
        .open(path)
        .map_err(|err| format!("cannot read '{path}': {err}"))?;
    let data = entry
        .data()
        .map_err(|err| format!("cannot read '{path}': {err}"))?;

    match mode {
        CatMode::Raw => out.write_all(data),
        CatMode::CStyle => write_c_dump(out, data),
        CatMode::Hex => write_hex_dump(out, data),
    }
    .map_err(write_error)
}

/// Writes a C-style dump: one offset column followed by 16 byte fields.
///
/// Every byte occupies three columns: two columns for its representation and
/// one separating space. Printable ASCII is shown as-is, common control
/// characters use their escape sequence (`\n`, `\t`, ...), and every other
/// byte is shown as two hexadecimal digits.
fn write_c_dump<W: Write>(out: &mut W, data: &[u8]) -> io::Result<()> {
    for (index, chunk) in data.chunks(DUMP_BYTES_PER_LINE).enumerate() {
        write!(out, "{:08x}", index * DUMP_BYTES_PER_LINE)?;
        for &byte in chunk {
            let repr = c_style_repr(byte);
            write!(out, " {repr:>2}")?;
        }
        writeln!(out)?;
    }
    Ok(())
}

fn c_style_repr(byte: u8) -> String {
    match byte {
        0x00 => "\\0".to_string(),
        0x07 => "\\a".to_string(),
        0x08 => "\\b".to_string(),
        b'\t' => "\\t".to_string(),
        b'\n' => "\\n".to_string(),
        0x0b => "\\v".to_string(),
        0x0c => "\\f".to_string(),
        b'\r' => "\\r".to_string(),
        0x20..=0x7e => (byte as char).to_string(),
        _ => format!("{byte:02x}"),
    }
}

/// Writes a hexadecimal dump: one offset column followed by 16 bytes.
fn write_hex_dump<W: Write>(out: &mut W, data: &[u8]) -> io::Result<()> {
    for (index, chunk) in data.chunks(DUMP_BYTES_PER_LINE).enumerate() {
        write!(out, "{:08x}", index * DUMP_BYTES_PER_LINE)?;
        for byte in chunk {
            write!(out, " {byte:02x}")?;
        }
        writeln!(out)?;
    }
    Ok(())
}

/// Prints `path` followed by all of its descendants, using `tree`-style
/// branch connectors.
fn cmd_tree<W: Write>(fs: &Fs<'_>, path: &str, dirs_only: bool, out: &mut W) -> Result<(), String> {
    let dir = fs
        .open(path)
        .map_err(|err| format!("cannot open '{path}': {err}"))?;
    if !dir.is_dir() {
        return Err(format!("'{path}' is not a directory"));
    }

    let mut rows = vec![Row::new(dir.name(), &dir)];
    collect_tree(&dir, "", dirs_only, &mut rows)?;
    render_table(out, rows)
}

/// Appends the descendants of `dir` to `rows` in depth-first order.
///
/// `prefix` contains the indentation of the parent, including `│   ` segments
/// for ancestors that still have following siblings.
fn collect_tree(
    dir: &Entry<'_>,
    prefix: &str,
    dirs_only: bool,
    rows: &mut Vec<Row>,
) -> Result<(), String> {
    let children: Vec<Entry<'_>> = dir
        .children()
        .map_err(|err| format!("cannot read directory: {err}"))?
        .filter(|entry| !dirs_only || entry.is_dir())
        .collect();
    let count = children.len();

    for (index, child) in children.iter().enumerate() {
        let last = index + 1 == count;
        let branch = if last { "└── " } else { "├── " };
        let mut name = String::with_capacity(prefix.len() + branch.len());
        name.push_str(prefix);
        name.push_str(branch);
        name.push_str(&String::from_utf8_lossy(child.name()));
        rows.push(Row::new(&name, child));

        if child.is_dir() {
            let continuation = if last { "    " } else { "│   " };
            let child_prefix = format!("{prefix}{continuation}");
            collect_tree(child, &child_prefix, dirs_only, rows)?;
        }
    }

    Ok(())
}

/// One row of the listing table.
struct Row {
    name: String,
    is_dir: bool,
    size: usize,
}

impl Row {
    fn new(name: impl AsRef<[u8]>, entry: &Entry<'_>) -> Self {
        Row {
            name: String::from_utf8_lossy(name.as_ref()).into_owned(),
            is_dir: entry.is_dir(),
            size: entry.size(),
        }
    }
}

/// Renders rows as a borderless table with a header.
fn render_table<W: Write>(out: &mut W, rows: Vec<Row>) -> Result<(), String> {
    let mut table = Table::new();
    table.load_style(NOTHING);
    table.set_header(["Name", "Type", "Size"]);
    for (index, column) in table.column_iter_mut().enumerate() {
        column.set_padding(if index == 2 { (0, 0) } else { (0, 2) });
    }
    if let Some(column) = table.column_mut(2) {
        column.set_cell_alignment(CellAlignment::Right);
    }

    for row in rows {
        table.add_row([
            Cell::new(row.name),
            Cell::new(if row.is_dir { "D" } else { "F" }),
            Cell::new(row.size),
        ]);
    }

    writeln!(out, "{table}").map_err(write_error)
}

fn write_error(err: io::Error) -> String {
    format!("failed to write output: {err}")
}
