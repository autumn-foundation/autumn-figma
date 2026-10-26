//! `autumn-figma`: turn a Figma design into Maud components for
//! Autumn apps.
//!
//! The crate ships the `autumn-figma` CLI and this library. The CLI has two
//! steps and a check:
//!
//! 1. `pull` reads the Figma REST API and writes a JSON snapshot.
//! 2. `generate` reads the snapshot and `figma.toml`. It writes Rust
//!    modules and one CSS file. It is offline and deterministic.
//! 3. `generate --check` writes nothing. It fails when the generated files
//!    do not agree with the snapshot.
//!
//! This crate is a development tool, not a runtime `Plugin`. Install it
//! with `cargo install`. `autumn plugin add` and the plugin index do not
//! apply to it.
//!
//! The generated code depends on `maud` only.
//!
//! ```no_run
//! use autumn_figma::{config::Config, generate};
//!
//! let config = Config::load("figma.toml".as_ref())?;
//! let output = generate::generate(&config)?;
//! output.write()?;
//! # Ok::<(), autumn_figma::Error>(())
//! ```
//!
//! ## Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`config`] | `figma.toml`, bindings, and the `FIGMA_TOKEN` lookup |
//! | [`model`] | Figma REST payloads |
//! | [`names`] | Safe identifiers, slugs, and literals |
//! | [`assets`] | Layers that export as SVG or PNG |
//! | [`diagnostics`] | Notes about the design |
//! | [`ir`] | The intermediate representation |
//! | [`client`] | Figma REST client |
//! | [`pull`] | Fetch snapshots and assets |
//! | [`tokens`] | Variables and styles to CSS custom properties |
//! | [`style`] | One node to CSS declarations |
//! | [`lower`] | The Figma tree to the intermediate representation |
//! | [`emit`] | The intermediate representation to Rust and CSS text |
//! | [`generate`] | Orchestration, write, and drift check |
//! | [`cli`] | The `autumn-figma` command line |

#![forbid(unsafe_code)]

pub mod assets;
pub mod cli;
pub mod client;
pub mod config;
pub mod diagnostics;
pub mod emit;
pub mod error;
pub mod generate;
pub mod ir;
pub mod lower;
pub mod model;
pub mod names;
pub mod pull;
pub mod style;
pub mod tokens;

pub use error::{Error, ErrorKind, Result};

#[cfg(test)]
mod test_support;
