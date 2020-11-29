#![allow(unused_imports, dead_code, unused_variables, unused_mut)]
extern crate ffmpeg_next as ffmpeg;

use std::{fs::DirBuilder, iter::repeat, path::PathBuf};

use anyhow::Result;
use ffmpeg::util::log as ffmpeg_log;
use thiserror::Error as ThisError;

pub mod files;
pub mod opts;
pub mod screencaps;
pub mod seek;
pub mod video;

#[derive(Debug, ThisError)]
enum Error {
    #[error("File {0} has no video stream.")]
    NoVideoStream(PathBuf),
    #[error("Could not read video stream for file {0}. Error: {1}")]
    CorruptVideoStream(PathBuf, ffmpeg::util::error::Error),
}

fn sandbox(opts: opts::Opts) -> Result<()> {
    let interval = 10;
    let start_at = 50;
    let items: Vec<usize> = repeat(1)
        .take(10)
        .enumerate()
        .map(|(i, _)| i * interval + start_at)
        .collect();
    dbg!(items);
    Ok(())
}

fn run(opts: opts::Opts) -> Result<()> {
    ffmpeg::init()?;
    if !opts.out_dir.exists() {
        log::info!(
            "Out directory {} doesn't exist. Creating...",
            opts.out_dir.to_str().unwrap()
        );
        DirBuilder::new()
            .recursive(true)
            .create(opts.out_dir.as_path())?;
    }
    let video_files = files::get_video_files_to_process(&opts)?;
    for path in video_files {
        let caps = screencaps::generate(&opts, path);
    }
    Ok(())
}

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let opts = opts::Opts::default();
    //TODO make configurable?
    ffmpeg_log::set_level(ffmpeg_log::Level::Panic);
    pretty_env_logger::init();

    //sandbox(opts)
    run(opts)
}
