#![allow(unused_imports, dead_code, unused_variables)]
extern crate ffmpeg_next as ffmpeg;

use std::{
    path::PathBuf,
    fs::DirBuilder,
};

use anyhow::Result;
use thiserror::Error as ThisError;

pub mod opts;
//pub mod screencaps;
pub mod video;
pub mod files;
pub mod util;

#[derive(Debug, ThisError)]
enum Error {
    #[error("File {0} has no video stream.")]
    NoVideoStream(PathBuf),
    #[error("Could not read video stream for file {0}. Error: {1}")]
    CorruptVideoStream(PathBuf, ffmpeg::util::error::Error),
}

fn sandbox() -> Result<()> {
    dotenv::dotenv().ok();
    pretty_env_logger::init();
    let opts = opts::Opts::default();
    Ok(())
}

fn run() -> Result<()> {
    dotenv::dotenv().ok();
    let opts = opts::Opts::default();
    pretty_env_logger::init();
    ffmpeg::init()?;
    if !opts.out_dir.exists() {
        log::info!("Out directory {} doesn't exist. Creating...", opts.out_dir.to_str().unwrap());
        DirBuilder::new().recursive(true).create(opts.out_dir.as_path())?;
    }
    let video_files = files::get_video_files_to_process(&opts)?;
    for path in video_files {
        //let caps = screencaps::generate(path);
    }
    Ok(())
}

fn main() -> Result<()> {
    //sandbox()
    run()
}
