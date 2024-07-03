use crate::{
    cli::{self, MultiProgressExt as _},
    files, screencaps,
    settings::Settings,
    util::ENV,
    Result,
};
use indicatif::ProgressBar;
use rayon::{prelude::*, ThreadPoolBuilder};
use std::{
    iter,
    path::{Path, PathBuf},
};

#[allow(clippy::panicking_unwrap)]
pub fn process_video<P: AsRef<Path>>(
    pbar: &ProgressBar,
    settings: &Settings,
    path: &P,
) -> Result<()> {
    let filename = files::get_filename(path);
    let path = path.as_ref();
    if !path.exists() {
        pbar.set_style(cli::ERROR_PROGRESS_BAR_STYLE.clone());
        log::error!("File {} does not exist.", filename);
        pbar.abandon_with_message(format!("File {} does not exist.", filename));
    } else {
        let result = screencaps::generate(pbar, settings, path);
        if ENV.unwrap_errors() && result.is_err() {
            result.unwrap();
        } else if let Err(error) = result {
            pbar.set_style(cli::ERROR_PROGRESS_BAR_STYLE.clone());
            log::error!("{} failed: {}", filename, error);
            pbar.abandon_with_message(format!("{} failed: {}", filename, error));
        }
    }
    Ok(())
}

pub fn rayon_process_videos(settings: &Settings, video_files: Vec<PathBuf>) -> Result<()> {
    let mp = cli::default_multi_progress()?;
    let items = video_files.into_iter().zip(iter::from_fn(|| {
        Some(mp.new_default_progress_bar(settings))
    }));
    if settings.synchronous() {
        //items.try_for_each(|(path, pbar)| process_video(&pbar, settings, &path))
        items
            .map(|(path, pbar)| process_video(&pbar, settings, &path))
            .collect::<Result<Vec<_>>>()
    } else {
        if let Some(threads) = settings.threads() {
            ThreadPoolBuilder::new()
                .num_threads(threads as usize)
                .build_global()
                .unwrap();
        }
        items
            .par_bridge()
            .map(|(path, pbar)| process_video(&pbar, settings, &path))
            .collect::<Result<Vec<_>>>()
    }?;
    Ok(())
}
