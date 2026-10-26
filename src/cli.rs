//! The `autumn-figma` command line.
//!
//! ```text
//! autumn-figma init --file-key <key or URL>
//! autumn-figma pull
//! autumn-figma generate [--check]
//! autumn-figma sync
//! ```
//!
//! Exit codes: [`EXIT_OK`], [`EXIT_ERROR`], [`EXIT_DRIFT`].

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};

/// Success.
pub const EXIT_OK: i32 = 0;
/// An error.
pub const EXIT_ERROR: i32 = 1;
/// `generate --check` found drift.
pub const EXIT_DRIFT: i32 = 2;

/// Turn a Figma design into Maud components, CSS tokens, and htmx-ready
/// fragments.
#[derive(Debug, Parser)]
#[command(name = "autumn-figma", version, about)]
pub struct Cli {
    /// The config file.
    #[arg(long, global = true, default_value = crate::config::CONFIG_FILE)]
    pub config: PathBuf,
    /// The command.
    #[command(subcommand)]
    pub command: Command,
}

/// A command.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Write a starter figma.toml.
    Init {
        /// The file key, or the Figma URL of the file.
        #[arg(long)]
        file_key: Option<String>,
    },
    /// Fetch the design snapshot and assets. Needs FIGMA_TOKEN.
    Pull,
    /// Generate Rust and CSS from the snapshot. Works offline.
    Generate {
        /// Write nothing. Exit 2 when the files differ from the snapshot.
        #[arg(long)]
        check: bool,
    },
    /// Run pull, then generate.
    Sync,
}

/// Run the CLI with `args` (including the program name).
///
/// `env` reads environment variables. `cwd` resolves a relative config
/// path. Normal output goes to `out`, notes and errors to `err`.
pub fn run(
    args: impl IntoIterator<Item = OsString>,
    env: &dyn Fn(&str) -> Option<String>,
    cwd: &Path,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let _ = (args.into_iter().count(), env, cwd, out, err);
    EXIT_ERROR
}

#[cfg(test)]
mod tests;
