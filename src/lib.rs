#![cfg_attr(docsrs, feature(doc_cfg))]
extern crate ffmpeg_next as ffmpeg;

#[macro_use]
extern crate log;

use std::fs::DirBuilder;

pub mod ffmpeg_ext;
pub mod files;
//pub mod opts;
pub mod screencaps;
pub mod settings;
pub mod util;
pub mod video;

pub mod cli;
mod error;
pub mod process;
pub use error::{Error, Result};

/// Run `mk-screens` using the provided options.
pub fn run(settings: &settings::Settings) -> Result<()> {
    ffmpeg::init()?;
    if !settings.out_dir().exists() {
        log::info!(
            "Out directory {} doesn't exist. Creating...",
            settings.out_dir().display()
        );
        DirBuilder::new()
            .recursive(true)
            .create(settings.out_dir())?;
    }
    debug!("Settings: {:#?}", settings);
    let video_files = files::get_video_files_to_process(settings)?;
    //process_videos(&settings, video_files)?;
    process::rayon_process_videos(settings, video_files)?;
    Ok(())
}
