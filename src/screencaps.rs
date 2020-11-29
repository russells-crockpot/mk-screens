use anyhow::Result;
use derivative::Derivative;
use ffmpeg::{
    decoder::Video as VideoDecoder,
    format::{context::Input, stream::Stream, Pixel},
    software::scaling::{context::Context as ScalingContext, flag::Flags},
    util::frame::video::Video,
};
use image::{codecs::jpeg::JpegEncoder, ColorType};
use mktemp::Temp;
use std::{
    fs::File,
    io::Write as _,
    ops::{Deref, DerefMut, Range},
    path::PathBuf,
};

use crate::{files::get_filename, opts::Opts, video::VidInfo};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct ScreenCap {
    time: i64,
    height: u32,
    width: u32,
    pixel_format: Pixel,
    #[derivative(Debug = "ignore")]
    data: Vec<u8>,
}

impl ScreenCap {
    pub fn new(range: Range<i64>, ts: i64, info: &mut VidInfo) -> Result<Self> {
        let height = info.height / 2;
        let width = info.width / 2;
        let cap = Self {
            height,
            width,
            time: ts,
            pixel_format: info.pixel_format,
            data: info.get_frame_at(range, ts, height, width)?,
        };
        Ok(cap)
    }

    pub fn save_file(&self, path: PathBuf) -> Result<()> {
        let mut file = File::create(path)?;
        let mut encoder = JpegEncoder::new(&mut file);
        let res = encoder.encode(
            &self.data,
            self.width as u32,
            self.height as u32,
            ColorType::Rgb8,
        );
        res.unwrap();
        Ok(())
    }
}

pub fn generate(opts: &Opts, path: PathBuf) -> Result<()> {
    log::info!("Generating screens for {}", get_filename(&path));
    let mut info = VidInfo::new(opts, path)?;
    let times = info.capture_times.clone();
    let captures: Vec<ScreenCap> = times
        .iter()
        .map(|(range, ts)| ScreenCap::new(range.clone(), *ts, &mut info).unwrap())
        .collect();
    for (i, cap) in captures.iter().enumerate() {
        let mut path = opts.out_dir.clone();
        path.push(format!("test-{}.jpeg", i));
        cap.save_file(path)?;
    }
    Ok(())
}
