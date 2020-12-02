use anyhow::Result;
use derivative::Derivative;
use ffmpeg::format::Pixel;
use image::{imageops, ImageFormat, RgbImage};
use indicatif::ProgressBar;
use itertools::Itertools as _;
use std::{fs::DirBuilder, path::PathBuf};

use crate::{
    files::get_file_stem, opts::Opts, util::envvar_to_bool, util::Dimensions, video::VidInfo,
};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct ScreenCap {
    pub time: i64,
    pub dimensions: Dimensions,
    pixel_format: Pixel,
    #[derivative(Debug = "ignore")]
    pub image: RgbImage,
}

impl ScreenCap {
    pub fn new(ts: i64, info: &mut VidInfo) -> Result<Self> {
        let (dimensions, frame_data) = info.get_frame_at(ts)?;
        let img = RgbImage::from_raw(dimensions.width(), dimensions.height(), frame_data).unwrap();
        Ok(Self {
            time: ts,
            dimensions: info.capture_dimensions.clone(),
            pixel_format: info.pixel_format,
            image: img,
        })
    }

    pub fn width(&self) -> u32 {
        self.dimensions.width()
    }
    pub fn height(&self) -> u32 {
        self.dimensions.height()
    }

    pub fn thumbnail(&self) -> RgbImage {
        imageops::thumbnail(&self.image, self.width(), self.height())
    }

    pub fn save_file(&self, path: PathBuf) -> Result<()> {
        log::debug!("Saving to file {}", path.to_str().unwrap());
        self.image.save_with_format(path, ImageFormat::Jpeg)?;
        Ok(())
    }
}

fn save_individual_img(opts: &Opts, cap: &ScreenCap, vidfile: &PathBuf, idx: usize) -> Result<()> {
    let mut out_path = opts.out_dir.clone();
    if envvar_to_bool("DIR_FOR_EACH_INDIVIDUAL_CAPTURES") {
        out_path.push(vidfile.file_stem().unwrap());
        if !out_path.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(opts.out_dir.as_path())?;
        }
    }
    out_path.push(format!(
        "{}-{}.jpeg",
        vidfile.file_stem().unwrap().to_str().unwrap(),
        idx
    ));
    cap.save_file(out_path)?;
    Ok(())
}

pub fn generate(pbar: &ProgressBar, opts: &Opts, path: PathBuf) -> Result<()> {
    //log::info!("Generating screens for {}", get_filename(&path));
    pbar.set_message(get_file_stem(&path));
    let mut info = VidInfo::new(opts, path.clone())?;
    pbar.inc(1);
    let save_individual_imgs = envvar_to_bool("SAVE_INDIVIDUAL_CAPTURES");
    let times = info.capture_times.clone();
    let Dimensions(cap_width, cap_height) = info.capture_dimensions;
    let mut img = RgbImage::new(cap_width * opts.columns, (cap_height + 2) * opts.rows);
    let mut current_x = 1;
    let mut current_y = 1;
    let captures = times
        .iter()
        .map(|ts| ScreenCap::new(*ts, &mut info).unwrap())
        .enumerate()
        .inspect(|_| pbar.inc(1))
        .chunks(opts.rows as usize);
    for chunk in &captures {
        for (idx, capture) in chunk {
            imageops::replace(&mut img, &capture.thumbnail(), current_x, current_y);
            if save_individual_imgs {
                save_individual_img(opts, &capture, &path, idx)?;
            }
            current_x += cap_width + 2;
        }
        current_y += cap_height + 2;
        current_x = 1;
    }
    let mut out_path = opts.out_dir.clone();
    out_path.push(info.img_file_name());
    img.save_with_format(out_path, ImageFormat::Jpeg)?;
    pbar.finish();
    Ok(())
}
