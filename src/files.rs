use filetime::{set_file_mtime, FileTime};
use std::{
    collections::HashMap,
    fs::{self, read_dir, remove_file, File},
    iter,
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::Result;

use crate::{opts::Opts, util::sync_mtimes};

pub fn get_filename<P: AsRef<Path>>(path: &P) -> &str {
    path.as_ref().file_name().unwrap().to_str().unwrap()
}

pub fn get_file_stem<P: AsRef<Path>>(path: &P) -> &str {
    path.as_ref().file_stem().unwrap().to_str().unwrap()
}

//TODO Add mime check
pub fn img_file_name<P: AsRef<Path>>(path: &P) -> String {
    format!("{}.jpg", get_filename(path))
}

struct FileInfo {
    video: Option<PathBuf>,
    screens: Option<PathBuf>,
}

impl FileInfo {
    pub fn for_video(opts: &Opts, path: &PathBuf) -> Self {
        let mut screens_path = opts.out_dir.clone();
        screens_path.push(img_file_name(&path));
        Self {
            video: Some(path.clone()),
            screens: if screens_path.exists() && !opts.force {
                Some(screens_path)
            } else {
                None
            },
        }
    }

    pub fn for_screens(video_files: &[PathBuf], path: &PathBuf) -> Self {
        Self {
            screens: Some(path.clone()),
            video: Self::find_video_file(video_files, path),
        }
    }

    pub fn with_video(&mut self, path: PathBuf) -> &mut Self {
        self.video = Some(path);
        self
    }

    pub fn with_screens(&mut self, opts: &Opts, path: PathBuf) -> &mut Self {
        if !opts.force {
            self.screens = Some(path);
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

    fn find_video_file(video_files: &[PathBuf], path: &PathBuf) -> Option<PathBuf> {
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

    pub fn add_video_file(&mut self, path: PathBuf) {
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

    pub fn add_screens_file(&mut self, path: PathBuf, video_files: &[PathBuf]) {
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
        .filter(mime_filter(&mime::VIDEO))
        .collect();
    video_files
        .iter()
        .for_each(|p| files.add_video_file(p.clone()));
    read_dir(opts.out_dir.as_path())?
        .map(|f| f.unwrap().path())
        .filter(|p| p.exists())
        .filter(mime_filter(&mime::IMAGE))
        .for_each(|p| files.add_screens_file(p, &video_files));
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
