use std::{
    env,
    fmt::{self, Display, Formatter},
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

pub fn envvar_to_bool(varname: &str) -> bool {
    match env::var(varname) {
        Err(_) => false,
        Ok(v) => usize::from_str(&v).unwrap() != 0,
    }
}
