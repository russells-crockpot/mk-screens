//! Items dealing with files.
use std::{
    collections::HashMap,
    fs::{self, read_dir, remove_file},
    iter,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::Result;

use crate::{opts::Opts, util::sync_mtimes};

/// A convenience function to get the file name from a path as a string.
pub fn get_filename<P: AsRef<Path>>(path: &P) -> &str {
    path.as_ref().file_name().unwrap().to_str().unwrap()
}

/// A convenience function to get the file stem from a path as a string.
pub fn get_file_stem<P: AsRef<Path>>(path: &P) -> &str {
    path.as_ref().file_stem().unwrap().to_str().unwrap()
}

/// Gets the file name to use for a screen capture based off of the original file name, which is
/// simple the file name suffixed with `.jpg`.
pub fn img_file_name<P: AsRef<Path>>(path: &P) -> String {
    format!("{}.jpg", get_filename(path))
}

struct FileInfo {
    video: Option<PathBuf>,
    screens: Option<PathBuf>,
}

impl FileInfo {
    pub fn for_video(opts: &Opts, path: &Path) -> Self {
        let mut screens_path = opts.out_dir.clone();
        screens_path.push(img_file_name(&path));
        Self {
            video: Some(path.into()),
            screens: if screens_path.exists() && !opts.force {
                Some(screens_path)
            } else {
                None
            },
        }
    }

    pub fn for_screens(video_files: &[PathBuf], path: &Path) -> Self {
        Self {
            screens: Some(path.into()),
            video: Self::find_video_file(video_files, path),
        }
    }

    pub fn with_video(&mut self, path: &Path) -> &mut Self {
        self.video = Some(path.into());
        self
    }

    pub fn with_screens(&mut self, opts: &Opts, path: &Path) -> &mut Self {
        if !opts.force {
            self.screens = Some(path.into());
        }
        self
    }

    pub fn should_delete_screens(&self) -> bool {
        self.video.is_none() && self.screens.is_some()
    }

    pub fn should_generate_screens(&self) -> Result<bool> {
        Ok(self.video.is_some()
            && (self.screens.is_none()
                || Self::modified_time(self.video.clone().unwrap())?
                    > Self::modified_time(self.screens.clone().unwrap())?))
    }

    fn find_video_file(video_files: &[PathBuf], path: &Path) -> Option<PathBuf> {
        let stem = get_file_stem(&path);
        for vid in video_files {
            if get_filename(vid) == stem {
                return Some(vid.clone());
            }
        }
        None
    }

    fn modified_time<P: AsRef<Path>>(path: P) -> Result<SystemTime> {
        Ok(fs::metadata(path)?.modified()?)
    }

    pub fn sync_mtimes(&self) -> Result<bool> {
        if self.video.is_none() || self.screens.is_none() {
            Ok(false)
        } else {
            Ok(sync_mtimes(
                self.video.as_ref().unwrap(),
                self.screens.as_ref().unwrap(),
            )?)
        }
    }

    pub fn screens(&self) -> Option<&PathBuf> {
        self.screens.as_ref()
    }
    pub fn video(&self) -> Option<&PathBuf> {
        self.video.as_ref()
    }
}

/// A special map that contains information on the video files to (possibly) generate screencaps for
/// as well as information about any screencps that already exist.
pub struct FileInfoMap<'a> {
    opts: &'a Opts,
    map: HashMap<String, FileInfo>,
}

impl<'a> FileInfoMap<'a> {
    pub fn new(opts: &'a Opts) -> Self {
        Self {
            opts,
            map: HashMap::new(),
        }
    }

    /// Adds a video file
    pub fn add_video_file(&mut self, path: &Path) {
        match self.map.get_mut(get_filename(&path)) {
            Some(info) => {
                info.with_video(path);
            }
            None => {
                self.map.insert(
                    String::from(get_filename(&path)),
                    FileInfo::for_video(&self.opts, &path),
                );
            }
        }
    }

    pub fn add_screens_file(&mut self, path: &Path, video_files: &[PathBuf]) {
        match self.map.get_mut(get_file_stem(&path)) {
            Some(info) => {
                info.with_screens(self.opts, path);
            }
            None => {
                self.map.insert(
                    String::from(get_filename(&path)),
                    FileInfo::for_screens(video_files, &path),
                );
            }
        }
    }

    pub fn get_screens_to_delete(&self) -> Vec<&PathBuf> {
        self.map
            .values()
            .filter(|info| info.should_delete_screens())
            .map(|info| info.screens().unwrap())
            .collect()
    }

    pub fn get_videos_to_process(&self) -> Vec<PathBuf> {
        self.map
            .values()
            //TODO
            .filter(|info| info.should_generate_screens().unwrap())
            .map(|info| info.video().unwrap().clone())
            .collect()
    }

    pub fn remove(&mut self, file_name: &str) {
        self.map.remove(file_name);
    }

    pub fn fix_times(&self) -> Result<usize> {
        let mut num_fixed = 0;
        for finfo in self.map.values() {
            if finfo.sync_mtimes()? {
                num_fixed += 1;
            }
        }
        Ok(num_fixed)
    }
}

pub fn mime_filter(mime_type: &'static mime::Name<'static>) -> Box<dyn Fn(&PathBuf) -> bool> {
    let mime_type = *mime_type;
    Box::new(move |path| {
        let rval = mime_guess::from_path(path)
            .into_iter()
            .filter(|g| g.type_() == mime_type)
            .count();
        rval > 0
    })
}

pub fn get_video_files_to_process(opts: &Opts) -> Result<Vec<PathBuf>> {
    let mut files = FileInfoMap::new(opts);
    let video_filter = mime_filter(&mime::VIDEO);
    let video_files: Vec<PathBuf> = opts
        .input
        .iter()
        .map(|p| {
            if p.is_file() {
                iter::once(p.clone()).collect::<Vec<PathBuf>>()
            } else {
                match read_dir(p.as_path()) {
                    Ok(entries) => entries.map(|f| f.unwrap().path()).collect(),
                    Err(_) => iter::empty().collect(),
                }
            }
        })
        .flatten()
        .filter(|p| p.exists())
        .map(PathBuf::from)
        .filter(&video_filter)
        .collect();
    video_files.iter().for_each(|p| files.add_video_file(p));
    read_dir(opts.out_dir.as_path())?
        .map(|f| f.unwrap().path())
        .filter(|p| p.exists())
        .filter(&video_filter)
        .for_each(|p| files.add_screens_file(&p, &video_files));
    if !opts.keep_files {
        let to_delete = files.get_screens_to_delete();
        if !to_delete.is_empty() {
            log::info!(
                "Deleting {} screencap file(s) with no associated video file.",
                to_delete.len()
            );
            for path in to_delete {
                match remove_file(&path) {
                    Ok(_) => log::info!("Deleted {}", get_filename(&path)),
                    Err(e) => log::warn!("Failed to delete {}. Error: {}", get_filename(&path), e),
                }
            }
        }
    }
    if opts.fix_times {
        log::info!("Fixing modified times...");
        let num_fixed = files.fix_times()?;
        log::info!("Fixed modified time for {} file(s).", num_fixed);
    }
    Ok(files.get_videos_to_process())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_img_file_name() {
        assert_eq!(
            img_file_name(&PathBuf::from("./test/test1.txt")),
            "test1.txt.jpg"
        );
        assert_eq!(img_file_name(&PathBuf::from("test1.txt")), "test1.txt.jpg");
        assert_eq!(
            img_file_name(&PathBuf::from("/test/test1.txt")),
            "test1.txt.jpg"
        );
    }

    #[test]
    fn test_mime_filter() {
        let text_filter = mime_filter(&mime::TEXT);
        let test_vec_1 = vec!["file1.txt", "file2.html", "file3.txt"];
        let results_1 = test_vec_1
            .iter()
            .map(PathBuf::from)
            .filter(&text_filter)
            .count();
        assert_eq!(results_1, 3);
        let test_vec_2 = vec!["file1.mp3", "file2.mp4", "file3.exe"];
        let results_2 = test_vec_2
            .iter()
            .map(PathBuf::from)
            .filter(&text_filter)
            .count();
        assert_eq!(results_2, 0);
        let test_vec_3 = vec!["file1.mp3", "file2.txt", "file3.exe"];
        let results_3 = test_vec_3
            .iter()
            .map(PathBuf::from)
            .filter(&text_filter)
            .count();
        assert_eq!(results_3, 1);
    }
}
