use std::path::PathBuf;

use anyhow::{Error as AnyhowError, Result};
use derivative::Derivative;
use ffmpeg::{format::context::Input, media::Type as MediaType};

use crate::{files::img_file_name, opts::Opts, Error};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct VidInfo {
    pub path: PathBuf,
    pub duration: u32,
    pub height: u16,
    pub width: u16,
    #[derivative(Debug = "ignore")]
    pub input: Input,
    pub interval: u32,
}

impl VidInfo {
    pub fn new(opts: &Opts, path: PathBuf) -> Result<Self> {
        let input = ffmpeg::format::input(&path)?;
        let stream = if let Some(stream) = input.streams().best(MediaType::Video) {
            stream
        } else {
            return Err(AnyhowError::from(Error::NoVideoStream(path)));
        };
        let video = match stream.codec().decoder().video() {
            Ok(v) => v,
            Err(e) => return Err(AnyhowError::from(Error::CorruptVideoStream(path, e))),
        };
        let duration = stream.duration() as u32;
        let start_at: u32 = (duration as f64 * opts.skip) as u32;
        todo!();
        //Ok(Self {
        //path,
        //duration,
        //width: video.width() as u16,
        //height: video.height() as u16,
        //input,
        //})
    }

    pub fn img_file_name(&self) -> String {
        img_file_name(&self.path)
    }
}
