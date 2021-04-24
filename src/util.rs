//! Various utilities and helpers.

use filetime::{set_file_mtime, FileTime};
use std::{
    env,
    fmt::{self, Display, Formatter},
    fs,
    path::Path,
    str::FromStr as _,
};

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
pub fn sync_mtimes<S, T>(source_file: S, target_file: T) -> anyhow::Result<bool>
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::set_var;

    #[test]
    fn test_envvar_to_bool() {
        set_var("MK_SCREENS_TEST_VAR", "1");
        assert_eq!(envvar_to_bool("MK_SCREENS_TEST_VAR"), true);
        set_var("MK_SCREENS_TEST_VAR", "2");
        assert_eq!(envvar_to_bool("MK_SCREENS_TEST_VAR"), true);
        set_var("MK_SCREENS_TEST_VAR", "0");
        assert_eq!(envvar_to_bool("MK_SCREENS_TEST_VAR"), false);
        set_var("MK_SCREENS_TEST_VAR", "");
        assert_eq!(envvar_to_bool("MK_SCREENS_TEST_VAR"), false);
        set_var("MK_SCREENS_TEST_VAR", "bad-input");
        assert_eq!(envvar_to_bool("MK_SCREENS_TEST_VAR"), false);
    }
}
