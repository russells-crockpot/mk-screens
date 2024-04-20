#![cfg_attr(docsrs, feature(doc_cfg))]
extern crate ffmpeg_next as ffmpeg;

#[macro_use]
extern crate log;

use std::{
    fs::DirBuilder,
    iter,
    path::{Path, PathBuf},
};

use eyre::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use rayon::prelude::*;
use thiserror::Error as ThisError;

pub mod ffmpeg_ext;
pub mod files;
//pub mod opts;
pub mod screencaps;
pub mod settings;
pub mod util;
pub mod video;

#[derive(Debug, ThisError)]
/// Various errors used by `mk-screens`.
pub enum Error {
    #[error("File {0} has no video stream.")]
    NoVideoStream(PathBuf),
    #[error("Could not read video stream for file {0}. Error: {1}")]
    CorruptVideoStream(PathBuf, ffmpeg::util::error::Error),
    #[error("Could not find filter named {0}.")]
    NoSuchFilter(String),
}

fn error_style() -> Result<ProgressStyle> {
    Ok(ProgressStyle::default_bar()
        .template("[{eta:>5}] {bar:.red/red} {percent:3}% | {wide_msg}")?
        .progress_chars("███"))
}

#[allow(clippy::panicking_unwrap)]
fn process_video<P: AsRef<Path>>(
    pbar: &ProgressBar,
    settings: &settings::Settings,
    path: &P,
) -> Result<()> {
    let filename = files::get_filename(path);
    let path = path.as_ref();
    if !path.exists() {
        pbar.set_style(error_style()?);
        log::error!("File {} does not exist.", filename);
        pbar.abandon_with_message(format!("File {} does not exist.", filename));
        return Ok(());
    }
    let result = screencaps::generate::<P>(pbar, settings, path);
    if settings.unwrap_errors() && result.is_err() {
        result.unwrap();
    } else if let Err(error) = result {
        pbar.set_style(error_style()?);
        log::error!("{} failed: {}", filename, error);
        pbar.abandon_with_message(format!("{} failed: {}", filename, error));
    }
    Ok(())
}

fn rayon_process_videos(
    settings: &settings::Settings,
    mut video_files: Vec<PathBuf>,
) -> Result<()> {
    let settings = settings.clone();
    let mp = if util::envvar_to_bool("HIDE_PROGRESS_BARS") {
        MultiProgress::with_draw_target(ProgressDrawTarget::hidden())
    } else {
        MultiProgress::new()
    };
    //mp.set_move_cursor(true);
    let pstyle = ProgressStyle::default_bar()
        .template("[{eta:>5}] {bar:.cyan/blue} {percent:3}% | {wide_msg}")?;
    let create_pbar = || {
        let pbar = mp.add(ProgressBar::new((settings.num_captures() + 2) as u64));
        pbar.set_style(pstyle.clone());
        Some(pbar)
    };
    let items: Vec<(PathBuf, ProgressBar)> = video_files
        .drain(..)
        .zip(iter::from_fn(create_pbar))
        .collect();
    if settings.synchronous() {
        items
            .iter()
            .try_for_each(|(path, pbar)| process_video(pbar, &settings, path))
        //.collect::<Result<()>>()
    } else {
        items
            .par_iter()
            .map(|(path, pbar)| process_video(pbar, &settings, path))
            .collect::<Result<()>>()
    }?;
    Ok(())
}

/// Run `mk-screens` using the provided options.
pub fn run(settings: &settings::Settings) -> Result<()> {
    ffmpeg::init()?;
    if !settings.out_dir().exists() {
        log::info!(
            "Out directory {} doesn't exist. Creating...",
            settings.out_dir().to_str().unwrap()
        );
        DirBuilder::new()
            .recursive(true)
            .create(settings.out_dir())?;
    }
    debug!("Settings: {:#?}", settings);
    let video_files = files::get_video_files_to_process(settings)?;
    //process_videos(&settings, video_files)?;
    rayon_process_videos(settings, video_files)?;
    Ok(())
}
