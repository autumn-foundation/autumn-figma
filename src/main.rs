//! The `autumn-figma` binary. See [`autumn_plugin_figma::cli`].

use std::process::ExitCode;

fn main() -> ExitCode {
    let cwd = std::env::current_dir().unwrap_or_default();
    let env = |key: &str| std::env::var(key).ok();
    let code = autumn_plugin_figma::cli::run(
        std::env::args_os(),
        &env,
        &cwd,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    );
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}
