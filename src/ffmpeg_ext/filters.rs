use crate::Error;
use anyhow::Result;
use ffmpeg::{
    filter::{context::Context, Graph},
    util::error::Error as FFMpegError,
};
use ffmpeg_sys_next as ffmpeg_sys;
use std::ffi::CString;

pub trait LinkableFilterContext {
    fn link<'a>(&mut self, other: Context<'a>) -> Result<Context<'a>>;
}

impl LinkableFilterContext for Context<'_> {
    fn link<'a>(&mut self, mut other: Context<'a>) -> Result<Context<'a>> {
        unsafe {
            match ffmpeg_sys::avfilter_link(self.as_mut_ptr(), 0, other.as_mut_ptr(), 0) {
                s if s >= 0 => Ok(other),
                e => Err(anyhow::Error::from(FFMpegError::from(e))),
            }
        }
    }
}

pub trait LinkableGraph {
    fn link(&mut self, from: &str, to: &str) -> Result<()>;
    fn chain_link(&mut self, filters: &[&str]) -> Result<()>;
}

impl LinkableGraph for Graph {
    fn link(&mut self, from: &str, to: &str) -> Result<()> {
        unsafe {
            let from_s = CString::new(from).unwrap();
            let ff_ptr = ffmpeg_sys::avfilter_graph_get_filter(self.as_mut_ptr(), from_s.as_ptr());

            let ff = if ff_ptr.is_null() {
                return Err(anyhow::Error::from(Error::NoSuchFilter(String::from(to))));
            } else {
                ff_ptr
            };
            let to_s = CString::new(to).unwrap();
            let tf_ptr = ffmpeg_sys::avfilter_graph_get_filter(self.as_mut_ptr(), to_s.as_ptr());

            let tf = if tf_ptr.is_null() {
                return Err(anyhow::Error::from(Error::NoSuchFilter(String::from(to))));
            } else {
                tf_ptr
            };
            match ffmpeg_sys::avfilter_link(ff, 0, tf, 0) {
                s if s >= 0 => Ok(()),
                e => Err(anyhow::Error::from(FFMpegError::from(e))),
            }
        }
    }

    fn chain_link(&mut self, filters: &[&str]) -> Result<()> {
        for (from, to) in filters.iter().zip(filters.iter().skip(1)) {
            self.link(from, to)?;
        }
        Ok(())
    }
}
