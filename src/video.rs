use std::{iter::repeat, ops::Range, path::PathBuf};

use anyhow::{Error as AnyhowError, Result};
use derivative::Derivative;
use ffmpeg::{
    decoder::Video as VideoDecoder,
    format::{context::Input, stream::Stream, Pixel as PixelFormat},
    media::Type as MediaType,
    software::scaling::{context::Context as ScalingContext, flag::Flags as ScalingFlags},
    util::{frame::video::Video, rational::Rational},
};

use crate::{
    files::img_file_name,
    opts::Opts,
    seek::{Flags as SeekFlags, FrameSeekable as _},
    Error,
};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct VidInfo {
    pub path: PathBuf,
    pub duration: i64,
    pub pixel_format: PixelFormat,
    //pub time_base: Rational,
    pub height: u32,
    pub width: u32,
    pub video_stream_idx: usize,
    #[derivative(Debug = "ignore")]
    pub input: Input,
    #[derivative(Debug = "ignore")]
    pub capture_times: Vec<(Range<i64>, i64)>,
}

#[allow(clippy::type_complexity)]
fn get_ranges_fn(opts: &Opts, duration: i64) -> Box<dyn Fn((usize, bool)) -> (Range<i64>, i64)> {
    let start_at = (duration as f64 * opts.skip) as i64;
    let interval = ((duration - start_at) as f64 / opts.num_captures() as f64) as i64;
    Box::new(move |(i, _)| {
        let time = i as i64 * interval + start_at;
        let start = if i == 0 { 0 } else { time - interval + 1 };
        let end = if (time + interval) > duration {
            duration
        } else {
            time + interval - 1
        };
        (Range { start, end }, time)
    })
}

impl VidInfo {
    pub fn new(opts: &Opts, path: PathBuf) -> Result<Self> {
        let input = ffmpeg::format::input(&path)?;
        let stream = if let Some(stream) = input.streams().best(MediaType::Video) {
            stream
        } else {
            return Err(AnyhowError::from(Error::NoVideoStream(path)));
        };
        let video = match stream.codec().decoder().video() {
            Ok(v) => v,
            Err(e) => return Err(AnyhowError::from(Error::CorruptVideoStream(path, e))),
        };
        //let time_base = video.time_base();
        let duration = stream.duration();
        //let duration = stream.duration().rescale(video.time_base(), DEFAULT_TIME_BASE);
        let ranges_fn = get_ranges_fn(&opts, duration);
        let capture_times: Vec<(Range<i64>, i64)> = repeat(true)
            .take(opts.num_captures() as usize)
            .enumerate()
            .map(ranges_fn)
            .collect();
        Ok(Self {
            path,
            duration,
            pixel_format: video.format(),
            width: video.width(),
            height: video.height(),
            video_stream_idx: stream.index(),
            //time_base,
            input,
            capture_times,
        })
    }

    pub fn stream<'a>(&'a self) -> Result<Stream<'a>> {
        if let Some(stream) = self.input.streams().best(MediaType::Video) {
            Ok(stream)
        } else {
            Err(AnyhowError::from(Error::NoVideoStream(self.path.clone())))
        }
    }

    pub fn img_file_name(&self) -> String {
        img_file_name(&self.path)
    }

    pub fn create_decoder(&self) -> Result<VideoDecoder> {
        Ok(self.stream()?.codec().decoder().video()?)
    }

    pub fn get_frame_at(
        &mut self,
        range: Range<i64>,
        ts: i64,
        output_height: u32,
        output_width: u32,
    ) -> Result<Vec<u8>> {
        log::debug!("Getting frame at {}", ts);
        let mut decoder = self.create_decoder()?;
        self.input
            .seek_to_frame(self.video_stream_idx as i32, ts, SeekFlags::ANY)?;
        let mut frame = Video::new(self.pixel_format, self.width, self.height);
        // Done to prevent a borrow of self
        let video_stream_idx = self.video_stream_idx;
        self.input
            .packets()
            .filter_map(|(s, p)| {
                if s.index() == video_stream_idx {
                    Some(p)
                } else {
                    None
                }
            })
            .take_while(|packet| {
                decoder.send_packet(packet).unwrap();
                decoder.receive_frame(&mut frame).is_err()
            })
            .last();
        let mut rgb_frame = Video::new(PixelFormat::RGB24, output_width, output_height);
        let mut scaler = ScalingContext::get(
            self.pixel_format,
            self.width,
            self.height,
            PixelFormat::RGB24,
            output_width,
            output_height,
            ScalingFlags::BILINEAR,
        )?;
        scaler.run(&frame, &mut rgb_frame)?;
        Ok(rgb_frame.data(0).to_vec())
    }
}
