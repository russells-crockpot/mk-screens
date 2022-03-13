extern crate ffmpeg_next as ffmpeg;

use eyre::Result;
use ffmpeg::util::log as ffmpeg_log;

#[cfg(feature = "pretty-errors")]
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
    match mk_screens::settings::Settings::load() {
        Ok(settings) => mk_screens::run(&settings),
        Err(error) => {
            if error.is::<clap::Error>() {
                let error = error.downcast::<clap::Error>()?;
                if error.kind == clap::ErrorKind::HelpDisplayed {
                    println!("{}", error.message);
                    Ok(())
                } else {
                    Err(error.into())
                }
            } else {
                Err(error)
            }
        }
    }
}
