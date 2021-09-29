extern crate ffmpeg_next as ffmpeg;

use std::{
    fs::DirBuilder,
    iter,
    path::{Path, PathBuf},
    thread,
};

use anyhow::Result;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use rayon::prelude::*;
use thiserror::Error as ThisError;

pub mod ffmpeg_ext;
pub mod files;
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

fn error_style() -> ProgressStyle {
    ProgressStyle::default_bar()
        .template("[{eta:>5}] {bar:.red/red} {percent:3}% | {wide_msg}")
        .progress_chars("███")
}

fn process_video(pbar: &ProgressBar, settings: &settings::Settings, path: &Path) {
    if !path.exists() {
        pbar.set_style(error_style());
        pbar.abandon_with_message(&format!("File {} does not exist.", path.to_str().unwrap()))
    } else if let Err(error) = screencaps::generate(pbar, settings, path) {
        pbar.set_style(error_style());
        pbar.abandon_with_message(&format!(
            "{} failed: {}",
            files::get_file_stem(&path),
            error
        ));
    }
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
    mp.set_move_cursor(false);
    let pstyle = ProgressStyle::default_bar()
        .template("[{eta:>5}] {bar:.cyan/blue} {percent:3}% | {wide_msg}");
    let create_pbar = || {
        let pbar = mp.add(ProgressBar::new((settings.num_captures() + 2) as u64));
        pbar.set_style(pstyle.clone());
        Some(pbar)
    };
    let items: Vec<(PathBuf, ProgressBar)> = video_files
        .drain(..)
        .zip(iter::from_fn(create_pbar))
        .collect();
    thread::spawn(move || {
        if settings.synchronous() {
            items
                .iter()
                .for_each(|(path, pbar)| process_video(pbar, &settings, path));
        } else {
            items
                .par_iter()
                .for_each(|(path, pbar)| process_video(pbar, &settings, path));
        }
    });
    mp.join()?;
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
    let video_files = files::get_video_files_to_process(settings)?;
    //process_videos(&settings, video_files)?;
    rayon_process_videos(settings, video_files)?;
    Ok(())
}
