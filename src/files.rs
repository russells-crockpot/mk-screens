//! Items dealing with files.
use std::{
    cell::RefCell,
    collections::HashMap,
    fs::{self, read_dir, remove_file},
    io::{BufRead as _, BufReader},
    iter,
    path::{Path, PathBuf},
    time::SystemTime,
};

use eyre::Result;
use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::{settings::Settings, util::sync_mtimes};

/// A convenience function to get the file name from a path as a string.
pub fn get_filename<P: AsRef<Path>>(path: P) -> String {
    if let Some(os_str) = path.as_ref().file_name() {
        os_str
            .to_str()
            .map(String::from)
            .unwrap_or_else(|| String::from("INVALID NAME"))
    } else {
        String::from("INVALID NAME")
    }
}

/// A convenience function to get the file stem from a path as a string.
pub fn get_file_stem<P: AsRef<Path>>(path: P) -> String {
    if let Some(os_str) = path.as_ref().file_stem() {
        os_str
            .to_str()
            .map(String::from)
            .unwrap_or_else(|| String::from("INVALID NAME"))
    } else {
        String::from("INVALID NAME")
    }
}

/// Gets the file name to use for a screen capture based off of the original file name, which is
/// simple the file name suffixed with `.jpg`.
pub fn img_file_name<P: AsRef<Path>>(path: &P) -> String {
    format!("{}.jpg", get_filename(path))
    //format!("{}.webp", get_filename(path))
}

struct FileInfo {
    video: Option<PathBuf>,
    screens: Option<PathBuf>,
}

impl FileInfo {
    pub fn for_video<P: AsRef<Path>>(settings: &Settings, path: P) -> Self {
        let mut screens_path = settings.out_dir().to_path_buf();
        screens_path.push(img_file_name(&path));
        Self {
            video: Some(path.as_ref().into()),
            screens: if screens_path.exists() && !settings.force() {
                Some(screens_path)
            } else {
                None
            },
        }
    }

    pub fn for_screens<P: AsRef<Path>>(video_files: &[PathBuf], path: P) -> Self {
        Self {
            screens: Some(path.as_ref().into()),
            video: Self::find_video_file(video_files, path),
        }
    }

    pub fn with_video<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.video = Some(path.as_ref().into());
        self
    }

    pub fn with_screens<P: AsRef<Path>>(&mut self, settings: &Settings, path: P) -> &mut Self {
        if !settings.force() {
            self.screens = Some(path.as_ref().into());
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

    fn find_video_file<P: AsRef<Path>>(video_files: &[PathBuf], path: P) -> Option<PathBuf> {
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
    settings: &'a Settings,
    map: HashMap<String, FileInfo>,
}

impl<'a> FileInfoMap<'a> {
    pub fn new(settings: &'a Settings) -> Self {
        Self {
            settings,
            map: HashMap::new(),
        }
    }

    /// Adds a video file to the map.
    pub fn add_video<P: AsRef<Path>>(&mut self, path: P) {
        match self.map.get_mut(&get_filename(&path)) {
            Some(info) => {
                info.with_video(path);
            }
            None => {
                self.map.insert(
                    get_filename(&path),
                    FileInfo::for_video(self.settings, path),
                );
            }
        }
    }

    pub fn add_screencap<P: AsRef<Path>>(&mut self, path: P, video_files: &[PathBuf]) {
        match self.map.get_mut(&get_file_stem(&path)) {
            Some(info) => {
                info.with_screens(self.settings, path);
            }
            None => {
                self.map.insert(
                    get_filename(&path),
                    FileInfo::for_screens(video_files, path),
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

/// Creates a closure that will filter out any files whose MIME type is not the specified type.
/// Useful for using with an iterator's `.filter()` method.
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

/// Gets a list of video files to process. A video file should be processed if:
/// 1. It has the MIME type of `video/*`.
/// 2. It doesn't already have a screencap file for it.
/// 3. If it does have a screencap file for it, then the video file must have been modified more
///    recently than the screencap file.
pub fn get_video_files_to_process(settings: &Settings) -> Result<Vec<PathBuf>> {
    let mut files = FileInfoMap::new(settings);
    let video_filter = mime_filter(&mime::VIDEO);
    let ignorer = Ignorer::new();
    let video_files: Vec<PathBuf> = settings
        .input()
        .iter()
        .flat_map(|p| {
            if p.is_file() {
                iter::once(p.clone()).collect::<Vec<PathBuf>>()
            } else {
                match read_dir(p.as_path()) {
                    Ok(entries) => entries.map(|f| f.unwrap().path()).collect(),
                    Err(_) => iter::empty().collect(),
                }
            }
        })
        .filter(|p| p.exists())
        .map(PathBuf::from)
        .filter(&video_filter)
        .filter(|p| !ignorer.should_ignore(p))
        .collect();
    video_files.iter().for_each(|p| files.add_video(p));
    read_dir(settings.out_dir())?
        .map(|f| f.unwrap().path())
        .filter(|p| p.exists())
        .filter(&video_filter)
        .for_each(|p| files.add_screencap(&p, &video_files));
    if !settings.keep_files() {
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
    if settings.fix_times() {
        log::info!("Fixing modified times...");
        let num_fixed = files.fix_times()?;
        log::info!("Fixed modified time for {} file(s).", num_fixed);
    }
    Ok(files.get_videos_to_process())
}

struct Ignorer(RefCell<HashMap<PathBuf, GlobSet>>);

impl Ignorer {
    fn new() -> Self {
        Self(RefCell::new(HashMap::new()))
    }

    fn load_ignore_file<P: AsRef<Path>>(&self, path_ref: P) {
        let mut ignore_file_path = path_ref.as_ref().to_path_buf();
        ignore_file_path.push(".mk-screens.ignore");
        let mut globs = GlobSetBuilder::new();
        if ignore_file_path.is_file() {
            log::debug!("Loading ignore file: {}", ignore_file_path.display());
            let reader = BufReader::new(fs::File::open(&ignore_file_path).unwrap());
            for (lineno, line) in reader.lines().map(Result::unwrap).enumerate() {
                match Glob::new(line.trim()) {
                    Ok(glob) => {
                        log::trace!("  Adding glob: {} (Regex: {})", glob.glob(), glob.regex());
                        globs.add(glob);
                    }
                    Err(err) => panic!(
                        "Invalid pattern on line {} of {}.\nError: {}",
                        lineno + 1,
                        ignore_file_path.display(),
                        err
                    ),
                }
            }
        }
        self.0
            .borrow_mut()
            .insert(path_ref.as_ref().to_path_buf(), globs.build().unwrap());
    }

    fn should_ignore<P: AsRef<Path>>(&self, path_ref: P) -> bool {
        let mut path = path_ref.as_ref().to_path_buf();
        while !path.is_dir() {
            if self.0.borrow().contains_key(&path) {
                break;
            } else if !path.exists() {
                self.0.borrow_mut().insert(path, GlobSet::empty());
                return false;
            } else if !path.pop() {
                unreachable!()
            }
        }
        if !self.0.borrow().contains_key(&path) {
            self.load_ignore_file(&path);
        }
        if !self.0.borrow().get(&path).unwrap().is_match(&path_ref) {
            if let Some(name) = path_ref.as_ref().file_name() {
                self.0.borrow().get(&path).unwrap().is_match(name)
            } else {
                false
            }
        } else {
            true
        }
    }
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
