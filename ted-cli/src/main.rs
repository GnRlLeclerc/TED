use std::{env::set_current_dir, fs::File, io};

use log::LevelFilter;
use simplelog::{Config, WriteLogger};

use clap::Parser;
use ted_tui::App;

use crate::args::Args;

mod args;
mod colors;

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Args::parse();

    WriteLogger::init(
        LevelFilter::Debug,
        Config::default(),
        File::create("debug.log").unwrap(),
    )
    .unwrap();

    if let Some(path) = args.path {
        if path.is_dir() {
            set_current_dir(path)?;
        } else if path.is_file() {
            // TODO
        } else {
            eprintln!("Path does not exist: {}", path.display());
        }
    }

    let mut app = App::new();
    app.run().await
}
