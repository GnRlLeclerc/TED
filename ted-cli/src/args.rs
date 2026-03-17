use std::path::PathBuf;

use clap::Parser;

use crate::colors::get_styles;

/// TUI Editor
#[derive(Parser)]
#[command(styles=get_styles())]
pub struct Args {
    /// File or path to open
    pub path: Option<PathBuf>,
}
