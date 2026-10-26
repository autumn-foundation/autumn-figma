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

use crate::client::FigmaClient;
use crate::config::{self, Config};
use crate::diagnostics::Level;
use crate::error::{Error, ErrorKind, Result};
use crate::generate::{self, Drift};
use crate::pull::{self, PullReport};

/// Success.
pub const EXIT_OK: i32 = 0;
/// An error.
pub const EXIT_ERROR: i32 = 1;
/// `generate --check` found drift.
pub const EXIT_DRIFT: i32 = 2;

/// Turn a Figma design into Maud components, CSS tokens, and htmx-ready
/// fragments.
#[derive(Debug, Parser)]
#[command(name = "autumn-figma", version, after_help = AFTER_HELP)]
pub struct Cli {
    /// The config file.
    #[arg(long, global = true, default_value = crate::config::CONFIG_FILE)]
    pub config: PathBuf,
    /// Show warnings and errors only.
    #[arg(long, short, global = true)]
    pub quiet: bool,
    /// The command.
    #[command(subcommand)]
    pub command: Command,
}

const AFTER_HELP: &str = "\
Paths in figma.toml are relative to the file.

Environment:
  FIGMA_TOKEN        Personal access token (scope file_content:read). Only `pull` needs it.
  FIGMA_OAUTH_TOKEN  OAuth token, when FIGMA_TOKEN is not set.
  FIGMA_API_BASE     API base URL (https, or http to localhost).

Exit codes:
  0  success
  1  error
  2  `generate --check` found generated files that differ";

/// A command.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Write a starter figma.toml.
    Init {
        /// The file key, or the Figma URL of the file.
        #[arg(long)]
        file_key: Option<String>,
    },
    /// Fetch the design snapshot and assets. Needs `FIGMA_TOKEN`.
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
    let cli = match Cli::try_parse_from(args) {
        Ok(cli) => cli,
        Err(e) => {
            use clap::error::ErrorKind as Clap;
            let text = e.render().to_string();
            return if matches!(e.kind(), Clap::DisplayHelp | Clap::DisplayVersion) {
                let _ = write!(out, "{text}");
                EXIT_OK
            } else {
                let _ = write!(err, "{text}");
                EXIT_ERROR
            };
        }
    };
    let config_path = cwd.join(&cli.config);
    let mut io = Io {
        out,
        err,
        quiet: cli.quiet,
        cwd,
    };
    let result = match cli.command {
        Command::Init { file_key } => init(&config_path, file_key.as_deref(), &mut io),
        Command::Pull => load(&config_path).and_then(|c| pull_step(&c, env, &mut io)),
        Command::Generate { check } => {
            load(&config_path).and_then(|c| generate_step(&c, check, &mut io))
        }
        Command::Sync => load(&config_path).and_then(|c| {
            pull_step(&c, env, &mut io)?;
            generate_step(&c, false, &mut io)
        }),
    };
    match result {
        Ok(code) => code,
        Err(e) => {
            io.line_err(&format!("error: {}", crate::names::printable(e.message())));
            EXIT_ERROR
        }
    }
}

struct Io<'a> {
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
    quiet: bool,
    cwd: &'a Path,
}

impl Io<'_> {
    fn line(&mut self, text: &str) {
        let _ = writeln!(self.out, "{text}");
    }

    fn line_err(&mut self, text: &str) {
        let _ = writeln!(self.err, "{text}");
    }
}

/// `path` relative to `root`, with `/` separators.
fn rel(root: &Path, path: &Path) -> String {
    let short = path.strip_prefix(root).unwrap_or(path);
    short
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn load(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Err(Error::new(
            ErrorKind::Config,
            format!(
                "{} not found; run `autumn-figma init --file-key <key or URL>` first, or pass --config",
                path.display()
            ),
        ));
    }
    Config::load(path)
}

fn init(path: &Path, file_key: Option<&str>, io: &mut Io<'_>) -> Result<i32> {
    if path.exists() {
        return Err(Error::new(
            ErrorKind::Config,
            format!("{} exists; init does not overwrite it", path.display()),
        ));
    }
    let key = match file_key {
        Some(k) => config::parse_file_key(k)?,
        None => config::KEY_PLACEHOLDER.to_owned(),
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| Error::io(dir, &e))?;
    }
    std::fs::write(path, config::starter_toml(&key)).map_err(|e| Error::io(path, &e))?;
    let shown = rel(io.cwd, path);
    io.line(&format!("wrote {shown}"));
    if file_key.is_none() {
        io.line("next: set file_key to the key or URL of your Figma file");
    }
    io.line(&format!(
        "next: export {} and run `autumn-figma sync`",
        config::TOKEN_ENV
    ));
    Ok(EXIT_OK)
}

fn pull_step(
    config: &Config,
    env: &dyn Fn(&str) -> Option<String>,
    io: &mut Io<'_>,
) -> Result<i32> {
    let token = config::token_from(env)?;
    let base = config::api_base_from(env)?;
    let client = FigmaClient::new(token, &base)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| Error::new(ErrorKind::Io, format!("async runtime: {e}")))?;
    let report: PullReport = runtime.block_on(pull::pull(config, &client))?;
    io.line(&format!("pulled {}", rel(&config.root, &report.snapshot)));
    if let Some(vars) = &report.variables {
        io.line(&format!("pulled {}", rel(&config.root, vars)));
    }
    for asset in &report.assets {
        io.line(&format!("pulled {}", rel(&config.root, asset)));
    }
    for notice in &report.notices {
        io.line_err(&format!("note: {}", crate::names::printable(notice)));
    }
    Ok(EXIT_OK)
}

fn generate_step(config: &Config, check: bool, io: &mut Io<'_>) -> Result<i32> {
    let output = generate::generate(config)?;
    for d in &output.diagnostics {
        if !(io.quiet && d.level == Level::Info) {
            io.line_err(&d.to_string());
        }
    }
    for asset in output.missing_assets(config) {
        io.line_err(&format!(
            "warning: missing asset {}; run `autumn-figma pull`",
            rel(&config.root, &asset)
        ));
    }
    if check {
        let drift = output.drift()?;
        if drift.is_empty() {
            io.line(&format!("in sync: {} files", output.files.len()));
            return Ok(EXIT_OK);
        }
        for d in &drift {
            let (what, path) = match d {
                Drift::Changed(p) => ("changed", p),
                Drift::Missing(p) => ("missing", p),
                Drift::Stale(p) => ("stale", p),
            };
            io.line_err(&format!("{what}: {}", rel(&config.root, path)));
        }
        io.line_err(
            "The generated files do not match the snapshot and figma.toml. Run `autumn-figma generate`, then commit the result.",
        );
        return Ok(EXIT_DRIFT);
    }
    let report = output.write()?;
    for path in &report.written {
        io.line(&format!("wrote {}", rel(&config.root, path)));
    }
    for path in &report.deleted {
        io.line(&format!("deleted {}", rel(&config.root, path)));
    }
    if !report.unchanged.is_empty() {
        io.line(&format!("{} files unchanged", report.unchanged.len()));
    }
    Ok(EXIT_OK)
}

#[cfg(test)]
mod tests;
