#![allow(unused_imports, dead_code, unused_variables)]
extern crate ffmpeg_next as ffmpeg;

use std::{fs::DirBuilder, path::PathBuf};

use anyhow::Result;
use thiserror::Error as ThisError;

pub mod opts;
//pub mod screencaps;
pub mod files;
pub mod video;

#[derive(Debug, ThisError)]
enum Error {
    #[error("File {0} has no video stream.")]
    NoVideoStream(PathBuf),
    #[error("Could not read video stream for file {0}. Error: {1}")]
    CorruptVideoStream(PathBuf, ffmpeg::util::error::Error),
}

fn sandbox(opts: opts::Opts) -> Result<()> {
    let v = vec![opts.input];
    let res: Vec<PathBuf> = v
        .iter()
        .map(PathBuf::from)
        .filter(files::mime_filter(&mime::VIDEO))
        .collect();
    dbg!(res);
    //let guesses = mime_guess::from_path(&opts.input)
    //.into_iter()
    //.filter(|t| { matches!(t.type_(), mime::VIDEO) })
    //.count();
    ////.for_each(|t| {dbg!(t);});
    //log::info!("{}", guesses);
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
    dbg!(&video_files);
    for path in video_files {
        //let caps = screencaps::generate(path);
    }
    Ok(())
}

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let opts = opts::Opts::default();
    pretty_env_logger::init();

    //sandbox(opts)
    run(opts)
}
