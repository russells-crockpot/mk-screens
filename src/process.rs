use crate::{
    cli::{self, MultiProgressExt as _},
    files, screencaps, settings, Result,
};
use indicatif::ProgressBar;
use rayon::prelude::*;
use std::{
    iter,
    path::{Path, PathBuf},
};

#[allow(clippy::panicking_unwrap)]
pub fn process_video<P: AsRef<Path>>(
    pbar: &ProgressBar,
    settings: &settings::Settings,
    path: &P,
) -> Result<()> {
    let filename = files::get_filename(path);
    let path = path.as_ref();
    if !path.exists() {
        pbar.set_style(cli::ERROR_PROGRESS_BAR_STYLE.clone());
        log::error!("File {} does not exist.", filename);
        pbar.abandon_with_message(format!("File {} does not exist.", filename));
        return Ok(());
    }
    let result = screencaps::generate::<P>(pbar, settings, path);
    if settings.unwrap_errors() && result.is_err() {
        result.unwrap();
    } else if let Err(error) = result {
        pbar.set_style(cli::ERROR_PROGRESS_BAR_STYLE.clone());
        log::error!("{} failed: {}", filename, error);
        pbar.abandon_with_message(format!("{} failed: {}", filename, error));
    }
    Ok(())
}

pub fn rayon_process_videos(
    settings: &settings::Settings,
    mut video_files: Vec<PathBuf>,
) -> Result<()> {
    let mp = cli::default_multi_progress()?;
    let items: Vec<(PathBuf, ProgressBar)> = video_files
        .drain(..)
        .zip(iter::from_fn(|| {
            Some(mp.new_default_progress_bar(settings))
        }))
        .collect();
    if settings.synchronous() {
        items
            .iter()
            .try_for_each(|(path, pbar)| process_video(pbar, settings, path))
        //.collect::<Result<()>>()
    } else {
        items
            .par_iter()
            .map(|(path, pbar)| process_video(pbar, settings, path))
            .collect::<Result<()>>()
    }?;
    Ok(())
}
