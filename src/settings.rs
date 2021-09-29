//!

use anyhow::{Error, Result};
use clap::{App, Arg, ArgMatches};
use config::{Config, File as ConfigFile};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn create_app<'a, 'b>() -> App<'a, 'b> {
    App::new(env!("CARGO_PKG_NAME"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("keep-files")
                .short("k")
                .long("keep-files")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("force")
                .short("f")
                .long("force")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("scale-up")
                .short("u")
                .long("scale-up")
                .takes_value(false)
                .help(concat!(
                    "If the video is smaller than the thumbnails would be, then scale up ",
                    "the thumbnail."
                )),
        )
        .arg(
            Arg::with_name("synchronous")
                .short("y")
                .long("synchronous")
                .takes_value(false)
                .help("Process only one video at a time."),
        )
        .arg(
            Arg::with_name("fix-times")
                .long("fix-times")
                .takes_value(false)
                .help("Fixes the modified time of any existing screens files."),
        )
        .arg(
            Arg::with_name("width")
                .takes_value(true)
                .short("w")
                .long("width"),
        )
        .arg(
            Arg::with_name("columns")
                .short("c")
                .long("columns")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("rows")
                .short("r")
                .long("rows")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("skip")
                .short("s")
                .long("skip")
                .takes_value(true)
                .help("The percent of amount of the video to skip at the beginning of the file."),
        )
        .arg(
            Arg::with_name("config")
                .long("config")
                .takes_value(true)
                .help(concat!(
                    "The configuration file to use. If provided, no other config files will ",
                    "be loaded."
                )),
        )
        .arg(
            Arg::with_name("out_dir")
                .short("o")
                .long("out_dir")
                .takes_value(true),
        )
        .arg(Arg::with_name("input").multiple(true))
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
    keep_files: bool,
    force: bool,
    scale_up: bool,
    synchronous: bool,
    width: u32,
    verbose: bool,
    columns: u32,
    rows: u32,
    skip: usize,
    fix_times: bool,
    out_dir: PathBuf,
    #[serde(skip_serializing)]
    input: Vec<PathBuf>,
}

impl Settings {
    pub fn load() -> Result<Self> {
        let mut conf = Self::get_default_config()?;
        let args = create_app().get_matches_safe()?;
        if let Some(config_file) = args.value_of("config") {
            Self::load_from_file(config_file, &mut conf)?;
        } else {
            Self::load_base_file(&mut conf)?;
            Self::load_from_file(".mk-screens.yaml", &mut conf)?;
        }
        Self::merge_cli_args(args, &mut conf)?;
        Ok(conf.try_into()?)
    }

    fn merge_cli_args(args: ArgMatches<'_>, conf: &mut Config) -> Result<()> {
        if args.is_present("keep-files") {
            conf.set("keep_files", true)?;
        }
        if args.is_present("force") {
            conf.set("force", true)?;
        }
        if args.is_present("scale-up") {
            conf.set("scale_up", true)?;
        }
        if args.is_present("synchronous") {
            conf.set("synchronous", true)?;
        }
        if args.is_present("verbose") {
            conf.set("verbose", true)?;
        }
        if args.is_present("fix-times") {
            conf.set("fix_times", true)?;
        }
        if let Some(width) = args.value_of("width") {
            conf.set("width", width)?;
        }
        if let Some(columns) = args.value_of("columns") {
            conf.set("columns", columns)?;
        }
        if let Some(rows) = args.value_of("rows") {
            conf.set("rows", rows)?;
        }
        if let Some(skip) = args.value_of("skip") {
            conf.set("skip", skip)?;
        }
        if let Some(out_dir) = args.value_of("out-dir") {
            conf.set("out_dir", out_dir)?;
        }
        if let Some(input) = args.values_of_lossy("input") {
            conf.set("input", input)?;
        } else {
            conf.set("input", vec!["."])?;
        }
        Ok(())
    }

    fn load_base_file(conf: &mut Config) -> Result<()> {
        let dirs = {
            let maybe_dirs = BaseDirs::new();
            if maybe_dirs.is_none() {
                //TODO maybe print a warning?
                return Ok(());
            }
            maybe_dirs.unwrap()
        };
        let config_dir = dirs.config_dir();
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }
        let mut config_path = PathBuf::from(config_dir);
        config_path.push("mk-screens.yaml");
        if !config_path.exists() {
            let to_save = conf.clone().try_into::<Self>()?;
            fs::write(&config_path, serde_yaml::to_vec(&to_save)?)?;
            Ok(())
        } else {
            Self::load_from_file(config_path, conf)
        }
    }

    fn load_from_file<P: AsRef<Path>>(path_ref: P, conf: &mut Config) -> Result<()> {
        let path = path_ref.as_ref();
        if path.exists() {
            if !path.is_file() {
                return Err(Error::msg(format!("{} is not a file!", path.display())));
            }
            conf.merge(ConfigFile::from(path))?;
        }
        Ok(())
    }

    fn get_default_config() -> Result<Config> {
        let mut conf = Config::new();
        conf.set_default("keep_files", false)?;
        conf.set_default("force", false)?;
        conf.set_default("scale_up", false)?;
        conf.set_default("synchronous", false)?;
        conf.set_default("verbose", false)?;
        conf.set_default("fix_times", false)?;
        conf.set_default("width", 3840)?;
        conf.set_default("columns", 12)?;
        conf.set_default("rows", 12)?;
        conf.set_default("skip", 5)?;
        conf.set_default("out_dir", "screens")?;
        Ok(conf)
    }

    pub fn num_captures(&self) -> u32 {
        (self.columns * self.rows) + 1
    }

    pub fn keep_files(&self) -> bool {
        self.keep_files
    }

    pub fn force(&self) -> bool {
        self.force
    }

    pub fn scale_up(&self) -> bool {
        self.scale_up
    }

    pub fn synchronous(&self) -> bool {
        self.synchronous
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn columns(&self) -> u32 {
        self.columns
    }

    pub fn rows(&self) -> u32 {
        self.rows
    }

    pub fn skip(&self) -> f64 {
        (self.skip as f64) / 100.0
    }

    pub fn fix_times(&self) -> bool {
        self.fix_times
    }

    pub fn out_dir(&self) -> &Path {
        self.out_dir.as_ref()
    }

    pub fn input(&self) -> &[PathBuf] {
        &self.input
    }
}
