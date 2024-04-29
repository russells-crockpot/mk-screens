extern crate ffmpeg_next as ffmpeg;

use ffmpeg::util::log as ffmpeg_log;
use mk_screens::{files, process, settings::Settings, Error, Result};
use std::fs::DirBuilder;

#[cfg(all(debug_assertions, feature = "pretty-errors"))]
fn _init_pretty_errors() -> Result<()> {
    if let Err(std::env::VarError::NotPresent) = std::env::var("RUST_BACKTRACE") {
        color_backtrace::BacktracePrinter::new()
            .verbosity(color_backtrace::Verbosity::Full)
            .install(color_backtrace::default_output_stream())
    } else {
        color_backtrace::install();
    }
    color_eyre::install()?;
    Ok(())
}

#[cfg(all(not(debug_assertions), feature = "pretty-errors"))]
fn _init_pretty_errors() -> Result<()> {
    color_backtrace::install();
    color_eyre::install()?;
    Ok(())
}

#[cfg(not(feature = "pretty-errors"))]
fn _init_pretty_errors() -> Result<()> {
    Ok(())
}

fn init() -> Result<()> {
    dotenv::dotenv().ok();
    //TODO make configurable?
    ffmpeg_log::set_level(ffmpeg_log::Level::Panic);
    _init_pretty_errors()?;
    //pretty_env_logger::init();
    ffmpeg::init()?;
    Ok(())
}

fn run(settings: &Settings) -> Result<()> {
    if !settings.out_dir().exists() {
        log::info!(
            "Out directory {} doesn't exist. Creating...",
            settings.out_dir().display()
        );
        DirBuilder::new()
            .recursive(true)
            .create(settings.out_dir())?;
    }
    log::debug!("Settings: {:#?}", settings);
    let video_files = files::get_video_files_to_process(settings)?;
    //process_videos(&settings, video_files)?;
    process::rayon_process_videos(settings, video_files)?;
    Ok(())
}

fn main() -> Result<()> {
    init()?;
    match Settings::load() {
        Ok(settings) => run(&settings),
        Err(error) => {
            if let Error::Clap { source: e } = error {
                //TODO
                if e.kind() == clap::error::ErrorKind::DisplayHelp {
                    e.print()?;
                    Ok(())
                } else {
                    Err(e.into())
                }
            } else {
                Err(error)
            }
        }
    }
}
