//! Contains items relevant to generating the screencap files.
use derivative::Derivative;
use eyre::Result;
use ffmpeg::format::Pixel;
use image::{imageops, ImageFormat, RgbImage};
use indicatif::ProgressBar;
#[cfg(target_family = "unix")]
use std::os::unix::fs::symlink;
#[cfg(target_family = "windows")]
use std::os::windows::fs::symlink_dir as symlink;
use std::{
    fs::{self, DirBuilder},
    path::{Path, PathBuf},
};
use unicode_width::UnicodeWidthStr;

use crate::{
    files::get_filename,
    settings::Settings,
    util::{safe_string_truncate, sync_mtimes, Dimensions, ENV},
    video::VidInfo,
};

const MAX_DISPLAY_NAME_WIDTH: usize = 80;

#[derive(Derivative)]
#[derivative(Debug)]
/// A representation of a single screen capture.
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
    pub fn save_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        log::info!("Saving to file {}", path.as_ref().display());
        self.image.save_with_format(path, ImageFormat::Jpeg)?;
        Ok(())
    }
}

/// Saves a [`ScreenCap`] as an individual file.
fn save_individual_img<P>(
    settings: &Settings,
    cap: &ScreenCap,
    vidfile: P,
    idx: usize,
) -> Result<()>
where
    P: AsRef<Path>,
{
    let vidfile = vidfile.as_ref();
    let mut out_path = settings.out_dir().to_path_buf();
    if ENV.dir_for_each_individual_captures() {
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

/// Returns the path of the image we should link to instead, or `None` is there is no such image
/// file.
fn get_image_to_link_to<P>(settings: &Settings, video_file: P) -> Option<PathBuf>
where
    P: AsRef<Path>,
{
    if !settings.allow_links() {
        log::trace!("Linking disabled");
        return None;
    }
    if !video_file.as_ref().is_symlink() {
        log::trace!("Video is not a symbolic link");
        return None;
    }
    let mut video_path = if let Ok(p) = fs::canonicalize(&video_file) {
        p
    } else {
        return None;
    };
    let filename = get_filename(&video_file);
    video_path.pop();
    video_path.push("screens");
    video_path.push(format!("{}.jpg", filename));
    if video_path.exists() {
        Some(video_path)
    } else {
        None
    }
}

fn finish_generation<P1, P2>(pbar: &ProgressBar, video_path: P1, image_path: P2) -> Result<()>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    sync_mtimes(video_path, image_path)?;
    pbar.finish_and_clear();
    Ok(())
}

/// Generates the screencap for a file and saves it.
pub fn generate<P>(pbar: &ProgressBar, settings: &Settings, path: P) -> Result<()>
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
    let mut info = VidInfo::new(settings, &path)?;
    pbar.inc(1);
    let mut out_path = settings.out_dir().to_path_buf();
    out_path.push(info.img_file_name());
    log::info!("Searching for image to link to for file {}", filename);
    if let Some(image_path) = get_image_to_link_to(settings, &path) {
        log::trace!(
            "Found image to link to {} for file {}",
            image_path.display(),
            filename
        );
        if let Err(e) = symlink(image_path, &out_path) {
            log::warn!("Could not link for {}: {}", filename, e);
        } else {
            log::trace!("Linked image for {}", filename);
            return finish_generation(pbar, path, out_path);
        }
    }
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
        if ENV.save_individual_captures() {
            save_individual_img(settings, &capture, &path, idx)?;
        }
        current_x += (cap_width + 2) as i64;
        if idx != 0 && idx as u32 % settings.columns() == 0 {
            current_y += (cap_height + 2) as i64;
            current_x = 1;
        }
    }
    img.save_with_format(out_path.clone(), ImageFormat::Jpeg)?;
    finish_generation(pbar, path, out_path)
}
