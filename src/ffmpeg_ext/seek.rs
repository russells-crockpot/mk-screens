use bitflags::bitflags;
use eyre::{Report, Result};
use ffmpeg::{format::context::Input, util::error::Error as FFMpegError};
use ffmpeg_sys_next as ffmpeg_sys;
use libc::c_int;

bitflags! {
    pub struct Flags: c_int {
        const ANY = ffmpeg_sys::AVSEEK_FLAG_ANY;
        const BACKWARD = ffmpeg_sys::AVSEEK_FLAG_BACKWARD;
        const BYTE = ffmpeg_sys::AVSEEK_FLAG_BYTE;
        const FRAME = ffmpeg_sys::AVSEEK_FLAG_FRAME;
    }
}

pub trait FrameSeekable {
    fn seek_to_frame(&mut self, stream_idx: i32, timestamp: i64, flags: Flags) -> Result<()>;
}

impl FrameSeekable for Input {
    fn seek_to_frame(&mut self, stream_idx: i32, timestamp: i64, flags: Flags) -> Result<()> {
        unsafe {
            match ffmpeg_sys::av_seek_frame(self.as_mut_ptr(), stream_idx, timestamp, flags.bits())
            {
                s if s >= 0 => Ok(()),
                e => Err(Report::from(FFMpegError::from(e))),
            }
        }
    }
}
