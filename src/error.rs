use clap::Error as ClapError;
use config::ConfigError;
use eyre::Report as ReportError;
use ffmpeg::util::error::Error as FfmpegError;
use log::SetLoggerError;
use serde_yaml::Error as YamlError;
use snafu::Snafu;
use std::{io::Error as IoError, path::PathBuf};

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(transparent)]
    Io { source: IoError },
    #[snafu(transparent)]
    SetLogger { source: SetLoggerError },
    #[snafu(transparent)]
    Yaml { source: YamlError },
    #[snafu(display("{} is not a file!", path.display()))]
    NotAFile { path: PathBuf },
    #[snafu(display("File {} has no video stream.", path.display()))]
    NoVideoStream { path: PathBuf },
    #[snafu(display("Could not find filter named {filter_name}."))]
    NoSuchFilter { filter_name: String },
    #[snafu(transparent)]
    Ffmpeg { source: FfmpegError },
    #[snafu(transparent)]
    Report { source: ReportError },
    #[snafu(transparent)]
    Clap { source: ClapError },
    #[snafu(transparent)]
    Config { source: ConfigError },
    #[snafu(whatever)]
    Other { message: String },
}

pub type Result<V> = std::result::Result<V, Error>;
