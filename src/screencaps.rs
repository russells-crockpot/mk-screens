use anyhow::Result;
use derivative::Derivative;
use ffmpeg::format::Pixel;
use image::{
    codecs::jpeg::JpegEncoder, imageops, ColorType, GenericImage as _, GenericImageView as _,
    ImageFormat, RgbImage,
};
use itertools::Itertools as _;
use std::{fs::File, path::PathBuf};

use crate::{files::get_filename, opts::Opts, video::VidInfo};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct ScreenCap {
    pub time: i64,
    pub height: u32,
    pub width: u32,
    pixel_format: Pixel,
    #[derivative(Debug = "ignore")]
    pub image: RgbImage,
}

impl ScreenCap {
    pub fn new(ts: i64, width: u32, height: u32, info: &mut VidInfo) -> Result<Self> {
        let frame_data = info.get_frame_at(ts)?;
        let img = RgbImage::from_raw(info.width, info.height, frame_data).unwrap();
        //let img = RgbImage::from_raw(info.width, info.height, frame_data).unwrap();
        Ok(Self {
            height,
            width,
            time: ts,
            pixel_format: info.pixel_format,
            image: imageops::thumbnail(&img, width, height),
        })
    }

    pub fn save_file(&self, path: PathBuf) -> Result<()> {
        log::debug!("Saving to file {}", path.to_str().unwrap());
        self.image.save_with_format(path, ImageFormat::Jpeg)?;
        Ok(())
    }
}

pub fn generate(opts: &Opts, path: PathBuf) -> Result<()> {
    log::info!("Generating screens for {}", get_filename(&path));
    let mut info = VidInfo::new(opts, path)?;
    let times = info.capture_times.clone();
    let capture_width = (info.width - (opts.columns * 4)) / opts.columns;
    let capture_height = ((capture_width as f64 / info.width as f64) * info.height as f64) as u32;
    let mut img = RgbImage::new(
        capture_width * opts.columns,
        (capture_height + 2) * opts.rows,
    );
    let mut current_x = 1;
    let mut current_y = 1;
    let captures = times
        .iter()
        .map(|ts| ScreenCap::new(*ts, capture_width, capture_height, &mut info).unwrap())
        .chunks(opts.rows as usize);
    //let captures: Vec<ScreenCap> = times
    //.iter()
    //.map(|ts| ScreenCap::new(*ts, capture_width, capture_height, &mut info).unwrap())
    //.collect();
    let mut idx = 0;
    for chunk in &captures {
        for capture in chunk {
            imageops::replace(&mut img, &capture.image, current_x, current_y);
            current_x += capture_width + 2;
        }
        current_y += capture_height + 2;
        current_x = 1;
    }
    let mut out_path = opts.out_dir.clone();
    out_path.push(info.img_file_name());
    img.save_with_format(out_path, ImageFormat::Jpeg)?;
    Ok(())
}
