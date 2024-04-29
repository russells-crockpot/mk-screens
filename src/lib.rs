#![cfg_attr(docsrs, feature(doc_cfg))]
extern crate ffmpeg_next as ffmpeg;

pub mod ffmpeg_ext;
pub mod files;
//pub mod opts;
pub mod screencaps;
pub mod settings;
pub mod util;
pub mod video;

pub mod cli;
mod error;
pub mod process;
pub use error::{Error, Result};
