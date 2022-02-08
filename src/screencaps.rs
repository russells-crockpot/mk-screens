//! Contains items relevant to generating the screencap files.
use anyhow::Result;
use derivative::Derivative;
use ffmpeg::format::Pixel;
use image::{imageops, ImageFormat, RgbImage};
use indicatif::ProgressBar;
use lazy_static::lazy_static;
use std::{fs::DirBuilder, path::Path};
use unicode_width::UnicodeWidthStr;

use crate::{
    files::get_filename,
    settings::Settings,
    util::{envvar_to_bool, safe_string_truncate, sync_mtimes, Dimensions},
    video::VidInfo,
};

const MAX_DISPLAY_NAME_WIDTH: usize = 20;

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
pub fn generate<P>(pbar: &ProgressBar, settings: &Settings, path: &Path) -> Result<()>
where
    P: AsRef<Path>,
{
    let filename = get_filename(&path);
    log::info!("Generating screens for {}", filename);
    let display_name = if filename.width() > MAX_DISPLAY_NAME_WIDTH {
        format!(
            "{}...",
            safe_string_truncate(&filename, MAX_DISPLAY_NAME_WIDTH)
        )
    } else {
        filename.clone()
    };
    pbar.set_message(display_name);
    log::debug!("Getting video info for {}", filename);
    let mut info = VidInfo::new(settings, path)?;
    pbar.inc(1);
    log::trace!("Generating capture times for {}", filename);
    let times = info.generate_capture_times(settings);
    log::trace!("Generated {} capture times for {}", times.len(), filename);
    let Dimensions(cap_width, cap_height) = info.capture_dimensions().clone();
    let mut img = RgbImage::new(
        cap_width * settings.columns(),
        (cap_height + 2) * settings.rows(),
    );
    let mut current_x = 1;
    let mut current_y = 1;
    let captures = times
        .iter()
        .inspect(|timestamp| {
            log::trace!(
                "Generating screencap for {} at time {}",
                filename,
                timestamp
            )
        })
        .map(|timestamp| ScreenCap::new(*timestamp, &mut info))
        .enumerate()
        .inspect(|_| pbar.inc(1));
    for (idx, maybe_capture) in captures {
        let capture = maybe_capture?;
        imageops::replace(&mut img, &capture.thumbnail(), current_x, current_y);
        if *SAVE_INDIVIDUAL_IMGS {
            save_individual_img(settings, &capture, path, idx)?;
        }
        current_x += (cap_width + 2) as i64;
        if idx != 0 && idx as u32 % settings.columns() == 0 {
            current_y += (cap_height + 2) as i64;
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
