use clap::Error as ClapError;
use config::ConfigError;
use eyre::Report as ReportError;
use ffmpeg::util::error::Error as FfmpegError;
use paste::paste;
use serde_yaml::Error as YamlError;
use snafu::Snafu;
use std::io::Error as IoError;

#[derive(Snafu, Debug)]
pub enum Error {
    Io {
        source: IoError,
    },
    Yaml {
        source: YamlError,
    },
    #[snafu(display("{path} is not a file!"))]
    NotAFile {
        path: String,
    },
    #[snafu(display("File {path} has no video stream."))]
    NoVideoStream {
        path: String,
    },
    #[snafu(display("Could not find filter named {filter_name}."))]
    NoSuchFilter {
        filter_name: String,
    },
    Ffmpeg {
        source: FfmpegError,
    },
    Report {
        source: ReportError,
    },
    Clap {
        source: ClapError,
    },
    Config {
        source: ConfigError,
    },
    #[snafu(display("{msg}"))]
    Other {
        msg: String,
    },
}
macro_rules! impl_from {
    ($type:path, $error:ident, $base_error:ident) => {
        impl From<$type> for $base_error {
            fn from(error: $type) -> Self {
                Self::$error { source: error }
            }
        }
    };
    ($type:path, $error:ident) => {
        impl_from! { $type, $error, Error }
    };
    ($name:ident) => {
        paste! {
            impl_from! { [<$name Error>], $name }
        }
    };
}

impl_from! {Yaml}
impl_from! {Io}
impl_from! {Ffmpeg}
impl_from! {Report}
impl_from! {Clap}
impl_from! {Config}

pub type Result<V> = std::result::Result<V, Error>;
