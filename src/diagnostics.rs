//! Notes that generation gives about the design.

use std::fmt;

/// How much a [`Diagnostic`] matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    /// Information. The output is as the design says.
    Info,
    /// The output differs from the design.
    Warning,
}

/// One note about one node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The level.
    pub level: Level,
    /// The layer path, for example `Screens > Todos > Title`.
    pub path: String,
    /// What happened.
    pub message: String,
}

impl Diagnostic {
    /// Make a warning.
    #[must_use]
    pub fn warning(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Warning,
            path: path.into(),
            message: message.into(),
        }
    }

    /// Make an information note.
    #[must_use]
    pub fn info(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            level: Level::Info,
            path: path.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let level = match self.level {
            Level::Info => "info",
            Level::Warning => "warning",
        };
        write!(f, "{level}: {}: {}", self.path, self.message)
    }
}
