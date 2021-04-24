extern crate ffmpeg_next as ffmpeg;

use anyhow::Result;
use ffmpeg::util::log as ffmpeg_log;

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    let opts = mk_screens::opts::Opts::default();
    //TODO make configurable?
    ffmpeg_log::set_level(ffmpeg_log::Level::Panic);
    pretty_env_logger::init();

    mk_screens::run(&opts)
}
