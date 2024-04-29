extern crate ffmpeg_next as ffmpeg;

use ffmpeg::util::log as ffmpeg_log;
use mk_screens::{settings::Settings, Error, Result};

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
    pretty_env_logger::init();
    Ok(())
}

fn main() -> Result<()> {
    init()?;
    match Settings::load() {
        Ok(settings) => mk_screens::run(&settings),
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
