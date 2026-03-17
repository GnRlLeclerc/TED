use std::{env::set_current_dir, io};

use clap::Parser;

use crate::args::Args;

mod args;
mod colors;

fn main() -> io::Result<()> {
    let args = Args::parse();

    if let Some(path) = args.path {
        if path.is_dir() {
            set_current_dir(path)?;
        } else if path.is_file() {
            // TODO
        } else {
            eprintln!("Path does not exist: {}", path.display());
        }
    }

    Ok(())
}
