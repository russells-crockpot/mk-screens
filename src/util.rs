use crate::Result;
use ffmpeg::util::log::Level as FfmpegLogLevel;
use filetime::{set_file_mtime, FileTime};
use std::{
    env,
    fmt::{self, Display, Formatter},
    fs,
    path::Path,
    str::FromStr as _,
    sync::OnceLock,
};
use unicode_width::UnicodeWidthStr;

lazy_static::lazy_static! {
    pub static ref ENV: EnvVars = EnvVars::default();
}

#[derive(Debug, Clone)]
pub struct Dimensions(pub u32, pub u32);

impl Dimensions {
    pub fn new(width: u32, height: u32) -> Self {
        Self(width, height)
    }
    pub fn width(&self) -> u32 {
        self.0
    }
    pub fn height(&self) -> u32 {
        self.1
    }
    pub fn area(&self) -> u32 {
        self.height() * self.width()
    }

    pub fn as_tuple(&self) -> (u32, u32) {
        (self.width(), self.height())
    }
}

impl Display for Dimensions {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}x{}", self.width(), self.height())
    }
}

impl PartialEq for Dimensions {
    fn eq(&self, other: &Dimensions) -> bool {
        self.width() == other.width() && self.height() == other.height()
    }
}

/// Gets the value of the provided environment variable name and converts it to a boolean value. A
/// environment variable's value is considered `true` if it's value is a positive, non-zero integer.
pub fn envvar_to_bool(varname: &str) -> bool {
    match env::var(varname) {
        Err(_) => false,
        Ok(v) => {
            if let Ok(n) = usize::from_str(&v) {
                n != 0
            } else {
                false
            }
        }
    }
}

/// Changes the last modified time of `target_file` to be the same as the last modified time of
/// `source_file`.
pub fn sync_mtimes<S, T>(source_file: S, target_file: T) -> Result<bool>
where
    S: AsRef<Path>,
    T: AsRef<Path>,
{
    let source_mtime = fs::metadata(source_file)?.modified()?;
    let target_mtime = fs::metadata(&target_file)?.modified()?;
    if source_mtime == target_mtime {
        return Ok(false);
    }
    let modified_time = FileTime::from_system_time(source_mtime);
    set_file_mtime(target_file, modified_time)?;
    Ok(true)
}

/// I know there's a better way to do this, but I don't want to deal with finding it right now.
pub fn safe_string_truncate(s: &str, target_size: usize) -> String {
    let mut s = String::from(s);
    let mut cur_width = s.width();
    while cur_width > target_size {
        if s.pop().is_none() {
            break;
        }
        cur_width = s.width();
    }
    s
}

#[derive(Debug, Default)]
pub struct EnvVars {
    hide_progress_bars: OnceLock<bool>,
    save_individual_captures: OnceLock<bool>,
    dir_for_each_individual_captures: OnceLock<bool>,
    unwrap_errors: OnceLock<bool>,
    ffmpeg_log_level: OnceLock<FfmpegLogLevel>,
    ffmpeg_probesize: OnceLock<String>,
    ffmpeg_analyzeduration: OnceLock<String>,
}

macro_rules! env_var_bool_getter {
    ($field:ident, $var_name:literal) => {
        pub fn $field(&self) -> bool {
            *self.$field.get_or_init(|| envvar_to_bool($var_name))
        }
    };
}

macro_rules! env_var_str_getter {
    ($field:ident, $var_name:literal, $default:literal) => {
        pub fn $field(&self) -> &str {
            self.$field
                .get_or_init(|| env::var($var_name).unwrap_or(String::from($default)))
        }
    };
}

impl EnvVars {
    env_var_bool_getter! {hide_progress_bars, "HIDE_PROGRESS_BARS"}
    env_var_bool_getter! {save_individual_captures, "SAVE_INDIVIDUAL_CAPTURES"}
    env_var_bool_getter! {dir_for_each_individual_captures, "DIR_FOR_EACH_INDIVIDUAL_CAPTURES"}
    env_var_bool_getter! {unwrap_errors, "UNWRAP_ERRORS"}
    env_var_str_getter! {ffmpeg_probesize, "FFMPEG_PROBESIZE", "250K"}
    env_var_str_getter! {ffmpeg_analyzeduration, "FFMPEG_ANALYZEDURATION", "25M"}

    pub fn ffmpeg_log_level(&self) -> FfmpegLogLevel {
        *self
            .ffmpeg_log_level
            .get_or_init(|| match env::var("FFMPEG_LOG_LEVEL") {
                Err(_) => FfmpegLogLevel::Panic,
                Ok(level_str) => match level_str.to_lowercase().as_ref() {
                    "quiet" => FfmpegLogLevel::Quiet,
                    "panic" => FfmpegLogLevel::Panic,
                    "fatal" => FfmpegLogLevel::Fatal,
                    "error" => FfmpegLogLevel::Error,
                    "warning" => FfmpegLogLevel::Warning,
                    "info" => FfmpegLogLevel::Info,
                    "verbose" => FfmpegLogLevel::Verbose,
                    "debug" => FfmpegLogLevel::Debug,
                    "trace" => FfmpegLogLevel::Trace,
                    other => {
                        log::warn!("Unknown ffmpeg log level: {}", other);
                        FfmpegLogLevel::Panic
                    }
                },
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::set_var;

    #[test]
    fn test_envvar_to_bool() {
        set_var("MK_SCREENS_TEST_VAR", "1");
        assert!(envvar_to_bool("MK_SCREENS_TEST_VAR"));
        set_var("MK_SCREENS_TEST_VAR", "2");
        assert!(envvar_to_bool("MK_SCREENS_TEST_VAR"));
        set_var("MK_SCREENS_TEST_VAR", "0");
        assert!(!envvar_to_bool("MK_SCREENS_TEST_VAR"));
        set_var("MK_SCREENS_TEST_VAR", "");
        assert!(!envvar_to_bool("MK_SCREENS_TEST_VAR"));
        set_var("MK_SCREENS_TEST_VAR", "bad-input");
        assert!(!envvar_to_bool("MK_SCREENS_TEST_VAR"));
    }
}
