extern crate ffmpeg_next as ffmpeg;

use anyhow::Result;
use ffmpeg::util::log as ffmpeg_log;

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let settings = mk_screens::settings::Settings::load()?;
    //TODO make configurable?
    ffmpeg_log::set_level(ffmpeg_log::Level::Panic);
    pretty_env_logger::init();
    mk_screens::run(&settings)
}
