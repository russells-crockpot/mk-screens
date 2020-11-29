use std::{
    collections::HashMap,
    fs::{read_dir, remove_file, File},
    path::{Path, PathBuf},
    time::SystemTime,
};

use anyhow::Result;

use crate::opts::Opts;

pub fn get_filename<P: AsRef<Path>>(path: &P) -> &str {
    path.as_ref().file_name().unwrap().to_str().unwrap()
}

pub fn get_file_stem<P: AsRef<Path>>(path: &P) -> &str {
    path.as_ref().file_stem().unwrap().to_str().unwrap()
}

//TODO Add mim check
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
            screens: if screens_path.exists() && !opts.overwrite {
                Some(screens_path)
            } else {
                None
            },
        }
    }

    pub fn for_screens(opts: &Opts, path: &PathBuf) -> Self {
        Self {
            screens: Some(path.clone()),
            video: Self::find_video_file(opts, path),
        }
    }

    pub fn with_video(&mut self, path: PathBuf) -> &mut Self {
        self.video = Some(path);
        self
    }

    pub fn with_screens(&mut self, opts: &Opts, path: PathBuf) -> &mut Self {
        if !opts.overwrite {
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

    fn find_video_file(opts: &Opts, path: &PathBuf) -> Option<PathBuf> {
        if opts.input.is_file() {
            if get_filename(&opts.input) == get_file_stem(&path) {
                Some(opts.input.clone())
            } else {
                None
            }
        } else {
            let mut source_path = opts.input.clone();
            source_path.push(get_file_stem(&path));
            if source_path.exists() {
                Some(source_path)
            } else {
                None
            }
        }
    }

    fn modified_time<P: AsRef<Path>>(path: P) -> Result<SystemTime> {
        Ok(File::open(path)?.metadata()?.modified()?)
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

    pub fn add_screens_file(&mut self, path: PathBuf) {
        match self.map.get_mut(get_file_stem(&path)) {
            Some(info) => {
                info.with_screens(self.opts, path);
            }
            None => {
                self.map.insert(
                    String::from(get_filename(&path)),
                    FileInfo::for_screens(&self.opts, &path),
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
            .filter(|info| info.should_generate_screens().unwrap())
            .map(|info| info.video().unwrap().clone())
            .collect()
    }

    pub fn remove(&mut self, file_name: &str) {
        self.map.remove(file_name);
    }
}

fn mime_filter(mime_type: &mime::Name) -> Box<dyn Fn(&PathBuf) -> bool> {
    Box::new(|path| {
        mime_guess::from_path(path)
            .into_iter()
            .filter(|g| matches!(g.type_(), mime_type))
            .count()
            > 0
    })
}

pub fn get_video_files_to_process(opts: &Opts) -> Result<Vec<PathBuf>> {
    let mut files = FileInfoMap::new(opts);
    if opts.input.is_file() {
        vec![opts.input.clone()]
    } else {
        read_dir(opts.input.as_path())?
            .map(|f| f.unwrap().path())
            .collect()
    }
    .iter()
    .map(PathBuf::from)
    .filter(mime_filter(&mime::VIDEO))
    .for_each(|p| files.add_video_file(p));
    read_dir(opts.out_dir.as_path())?
        .map(|f| f.unwrap().path())
        .filter(mime_filter(&mime::IMAGE))
        .for_each(|p| files.add_screens_file(p));
    if !opts.keep_files {
        let to_delete = files.get_screens_to_delete();
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
    Ok(files.get_videos_to_process())
}
