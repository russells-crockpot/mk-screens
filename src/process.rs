use crate::{cli, files, screencaps, settings, util, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget};
use rayon::prelude::*;
use std::{
    iter,
    path::{Path, PathBuf},
};

#[allow(clippy::panicking_unwrap)]
pub(crate) fn process_video<P: AsRef<Path>>(
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

pub(crate) fn rayon_process_videos(
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
    let create_pbar = || {
        let pbar = mp.add(ProgressBar::new((settings.num_captures() + 2) as u64));
        pbar.set_style(cli::PROGRESS_BAR_STYLE.clone().clone());
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
