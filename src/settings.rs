//!

use crate::{Error, Result};
use clap::Parser;
use config::{
    builder::{ConfigBuilder as BaseConfigBuilder, DefaultState},
    Config, File as ConfigFile,
};
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(version, author)]
#[command(rename_all = "kebab")]
#[command(about = env!("CARGO_PKG_DESCRIPTION"))]
pub struct Cli {
    #[arg(long)]
    keep_files: bool,
    #[arg(long)]
    force: bool,
    #[arg(short = 'e', long)]
    unwrap_errors: bool,
    #[arg(short, long)]
    verbose: bool,
    #[arg(
        short = 'u',
        long,
        help = "If the video is smaller than the thumbnails would be, then scale up the thumbnail."
    )]
    scale_up: bool,
    #[arg(short = 'y', long, help = "Process only one video at a time.")]
    synchronous: bool,
    #[arg(long, help = "Fixes the modified time of any existing screens files.")]
    fix_times: bool,
    #[arg(short, long)]
    width: Option<u32>,
    #[arg(short, long)]
    columns: Option<u32>,
    #[arg(short, long)]
    rows: Option<u32>,
    #[arg(
        short = 'i',
        long,
        help = "Write any failures to the .mk-screens.ignore file. (Unimplemented)"
    )]
    save_failures_to_ignore: bool,
    #[arg(
        short,
        long,
        help = "The percent of amount of the video to skip at the beginning of the file."
    )]
    skip: Option<u32>,
    #[arg(long)]
    config: Option<String>,
    #[arg(short, long)]
    out_dir: Option<String>,
    #[arg(default_value = ".")]
    input: Vec<String>,
}

type ConfigBuilder = BaseConfigBuilder<DefaultState>;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Settings {
    keep_files: bool,
    force: bool,
    unwrap_errors: bool,
    verbose: bool,
    scale_up: bool,
    synchronous: bool,
    fix_times: bool,
    width: u32,
    columns: u32,
    rows: u32,
    save_failures_to_ignore: bool,
    skip: usize,
    out_dir: PathBuf,
    #[serde(skip_serializing)]
    input: Vec<PathBuf>,
}

impl Settings {
    pub fn load() -> Result<Self> {
        let cli = Cli::parse();
        let mut conf_builder = Self::get_default_config()?;
        if let Some(ref config_file) = cli.config {
            conf_builder = Self::load_from_file(config_file, conf_builder)?;
        } else {
            conf_builder = Self::load_base_file(conf_builder)?;
            conf_builder = Self::load_from_file(".mk-screens.yaml", conf_builder)?;
        }
        conf_builder = Self::merge_cli_args(cli, conf_builder)?;
        Ok(conf_builder.build()?.try_deserialize()?)
    }

    fn merge_cli_args(cli: Cli, mut conf_builder: ConfigBuilder) -> Result<ConfigBuilder> {
        if cli.fix_times {
            conf_builder = conf_builder.set_override("fix_times", true)?;
        }
        if cli.force {
            conf_builder = conf_builder.set_override("force", true)?;
        }
        if cli.keep_files {
            conf_builder = conf_builder.set_override("keep_files", true)?;
        }
        if cli.save_failures_to_ignore {
            conf_builder = conf_builder.set_override("save_failures_to_ignore", true)?;
        }
        if cli.scale_up {
            conf_builder = conf_builder.set_override("scale_up", true)?;
        }
        if cli.synchronous {
            conf_builder = conf_builder.set_override("synchronous", true)?;
        }
        if cli.unwrap_errors {
            conf_builder = conf_builder.set_override("unwrap_errors", true)?;
        }
        if cli.verbose {
            conf_builder = conf_builder.set_override("verbose", true)?;
        }
        Ok(conf_builder
            .set_override_option("width", cli.width)?
            .set_override_option("columns", cli.columns)?
            .set_override_option("rows", cli.rows)?
            .set_override_option("skip", cli.skip)?
            .set_override_option("out_dir", cli.out_dir)?
            .set_override("input", cli.input)?)
    }

    fn load_base_file(conf_builder: ConfigBuilder) -> Result<ConfigBuilder> {
        let dirs = {
            let maybe_dirs = BaseDirs::new();
            if maybe_dirs.is_none() {
                //TODO maybe print a warning?
                return Ok(conf_builder);
            }
            maybe_dirs.unwrap()
        };
        let config_dir = dirs.config_dir();
        if !config_dir.exists() {
            fs::create_dir_all(config_dir)?;
        }
        let mut config_path = PathBuf::from(config_dir);
        config_path.push("mk-screens.yaml");
        if !config_path.exists() {
            let to_save = conf_builder.build_cloned()?.try_deserialize::<Self>()?;
            //let to_save = conf_builder.build_cloned().try_into()?;
            fs::write(&config_path, serde_yaml::to_vec(&to_save)?)?;
            Ok(conf_builder)
        } else {
            Self::load_from_file(config_path, conf_builder)
        }
    }

    fn load_from_file<P>(path_ref: P, conf_builder: ConfigBuilder) -> Result<ConfigBuilder>
    where
        P: AsRef<Path>,
    {
        let path = path_ref.as_ref();
        if path.exists() {
            if !path.is_file() {
                return Err(Error::NotAFile {
                    path: path.to_path_buf(),
                });
            }
            Ok(conf_builder.add_source(ConfigFile::from(path)))
        } else {
            Ok(conf_builder)
        }
    }

    fn get_default_config() -> Result<ConfigBuilder> {
        Ok(Config::builder()
            .set_default("keep_files", false)?
            .set_default("force", false)?
            .set_default("unwrap_errors", false)?
            .set_default("scale_up", false)?
            .set_default("synchronous", false)?
            .set_default("verbose", false)?
            .set_default("fix_times", false)?
            .set_default("save_failures_to_ignore", false)?
            .set_default("width", 3840)?
            .set_default("columns", 12)?
            .set_default("rows", 12)?
            .set_default("skip", 5)?
            .set_default("out_dir", "screens")?)
    }

    pub fn num_captures(&self) -> u32 {
        (self.columns * self.rows) + 1
    }

    pub fn keep_files(&self) -> bool {
        self.keep_files
    }

    pub fn unwrap_errors(&self) -> bool {
        self.unwrap_errors
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

    pub fn save_failures_to_ignore(&self) -> bool {
        self.save_failures_to_ignore
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
