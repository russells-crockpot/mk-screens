//! Contains items relevant to generating the screencap files.
use anyhow::Result;
use derivative::Derivative;
use ffmpeg::format::Pixel;
use image::{imageops, ImageFormat, RgbImage};
use indicatif::ProgressBar;
use lazy_static::lazy_static;
use std::{fs::DirBuilder, path::Path};

use crate::{
    files::{get_file_stem, get_filename},
    settings::Settings,
    util::{envvar_to_bool, sync_mtimes, Dimensions},
    video::VidInfo,
};

lazy_static! {
    static ref SAVE_INDIVIDUAL_IMGS: bool = envvar_to_bool("SAVE_INDIVIDUAL_CAPTURES");
    static ref DIR_FOR_EACH_CAP: bool = envvar_to_bool("DIR_FOR_EACH_INDIVIDUAL_CAPTURES");
}

#[derive(Derivative)]
#[derivative(Debug)]
/// A representation of a screencap file.
pub struct ScreenCap {
    timestamp: i64,
    dimensions: Dimensions,
    pixel_format: Pixel,
    #[derivative(Debug = "ignore")]
    image: RgbImage,
}

impl ScreenCap {
    pub fn new(timestamp: i64, info: &mut VidInfo) -> Result<Self> {
        let (dimensions, frame_data) = info.get_frame_at(timestamp)?;
        let img = RgbImage::from_raw(dimensions.width(), dimensions.height(), frame_data).unwrap();
        Ok(Self {
            timestamp,
            dimensions: info.capture_dimensions().clone(),
            pixel_format: info.pixel_format(),
            image: img,
        })
    }

    /// The underlying image.
    pub fn image(&self) -> &RgbImage {
        &self.image
    }

    /// The width of the final image (in pixels).
    pub fn width(&self) -> u32 {
        self.dimensions.width()
    }

    /// The height of the final image (in pixels).
    pub fn height(&self) -> u32 {
        self.dimensions.height()
    }

    fn thumbnail(&self) -> RgbImage {
        imageops::thumbnail(&self.image, self.width(), self.height())
    }

    /// Saves the generated screen capture to the provided file.
    pub fn save_file(&self, path: &Path) -> Result<()> {
        log::info!("Saving to file {}", path.to_str().unwrap());
        self.image.save_with_format(path, ImageFormat::Jpeg)?;
        Ok(())
    }
}

fn save_individual_img(
    settings: &Settings,
    cap: &ScreenCap,
    vidfile: &Path,
    idx: usize,
) -> Result<()> {
    let mut out_path = settings.out_dir().to_path_buf();
    if *DIR_FOR_EACH_CAP {
        out_path.push(vidfile.file_stem().unwrap());
        if !out_path.exists() {
            DirBuilder::new()
                .recursive(true)
                .create(settings.out_dir())?;
        }
    }
    out_path.push(format!(
        "{}-{}.jpeg",
        vidfile.file_stem().unwrap().to_str().unwrap(),
        idx
    ));
    cap.save_file(&out_path)?;
    Ok(())
}

/// Generates the screencap for a file and saves it.
pub fn generate(pbar: &ProgressBar, settings: &Settings, path: &Path) -> Result<()> {
    log::info!("Generating screens for {}", get_filename(&path));
    pbar.set_message(get_file_stem(&path));
    let mut info = VidInfo::new(settings, path)?;
    pbar.inc(1);
    let times = info.generate_capture_times(settings);
    let Dimensions(cap_width, cap_height) = info.capture_dimensions().clone();
    let mut img = RgbImage::new(
        cap_width * settings.columns(),
        (cap_height + 2) * settings.rows(),
    );
    let mut current_x = 1;
    let mut current_y = 1;
    let captures = times
        .iter()
        .map(|timestamp| ScreenCap::new(*timestamp, &mut info))
        .enumerate()
        .inspect(|_| pbar.inc(1));
    for (idx, maybe_capture) in captures.into_iter() {
        let capture = maybe_capture?;
        imageops::replace(&mut img, &capture.thumbnail(), current_x, current_y);
        if *SAVE_INDIVIDUAL_IMGS {
            save_individual_img(settings, &capture, path, idx)?;
        }
        current_x += cap_width + 2;
        if idx != 0 && idx as u32 % settings.columns() == 0 {
            current_y += cap_height + 2;
            current_x = 1;
        }
    }
    let mut out_path = settings.out_dir().to_path_buf();
    out_path.push(info.img_file_name());
    img.save_with_format(out_path.clone(), ImageFormat::Jpeg)?;
    sync_mtimes(path, out_path)?;
    pbar.finish_and_clear();
    Ok(())
}
