use anyhow::Result;
use derivative::Derivative;
use ffmpeg::format::Pixel;
use image::{codecs::jpeg::JpegEncoder, imageops::thumbnail, ColorType, ImageFormat, RgbImage};
use std::{fs::File, path::PathBuf};

use crate::{files::get_filename, opts::Opts, video::VidInfo};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct ScreenCap {
    time: i64,
    height: u32,
    width: u32,
    pixel_format: Pixel,
    #[derivative(Debug = "ignore")]
    image: RgbImage,
}

impl ScreenCap {
    pub fn new(ts: i64, width: u32, height: u32, info: &mut VidInfo) -> Result<Self> {
        let frame_data = info.get_frame_at(ts)?;
        let img = RgbImage::from_raw(info.width, info.height, frame_data).unwrap();
        Ok(Self {
            height,
            width,
            time: ts,
            pixel_format: info.pixel_format,
            image: img,
        })
    }

    pub fn save_file(&self, path: PathBuf) -> Result<()> {
        log::debug!("Saving to file {}", path.to_str().unwrap());
        let thumb = thumbnail(&self.image, self.width, self.height);
        thumb.save_with_format(path, ImageFormat::Jpeg)?;
        Ok(())
    }
}

pub fn generate(opts: &Opts, path: PathBuf) -> Result<()> {
    log::info!("Generating screens for {}", get_filename(&path));
    let mut info = VidInfo::new(opts, path)?;
    let times = info.capture_times.clone();
    let capture_width = (info.width - (opts.columns * 4)) / opts.columns;
    let capture_height = ((capture_width as f64 / info.width as f64) * info.height as f64) as u32;
    //let capture_width = info.width - 1;
    //let capture_height = info.height - 1;
    let captures: Vec<ScreenCap> = times
        .iter()
        .map(|ts| ScreenCap::new(*ts, capture_width, capture_height, &mut info).unwrap())
        .collect();
    for (i, cap) in captures.iter().enumerate() {
        let mut path = opts.out_dir.clone();
        path.push(format!("test-{}.jpeg", i));
        cap.save_file(path)?;
    }
    Ok(())
}
