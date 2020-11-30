use std::{iter::repeat, path::PathBuf};

use anyhow::{Error as AnyhowError, Result};
use derivative::Derivative;
use ffmpeg::{
    decoder::Video as VideoDecoder,
    filter::{self, Graph},
    format::{context::Input, stream::Stream, Pixel as PixelFormat},
    media::Type as MediaType,
    software::scaling::{context::Context as ScalingContext, flag::Flags as ScalingFlags},
    util::frame::video::Video,
};

use crate::{
    files::img_file_name,
    opts::Opts,
    seek::{Flags as SeekFlags, FrameSeekable as _},
    Error,
};

fn create_timestamp_filter(
    decoder: &VideoDecoder,
    stream: &Stream,
    out_width: u32,
    out_height: u32,
) -> Result<Graph> {
    let mut filter = Graph::new();
    let sar = decoder.aspect_ratio();
    let buffer_args = format!(
        "width={}:height={}:video_size={}x{}:pix_fmt={}:time_base={}:sar={}",
        out_width,
        out_height,
        decoder.width(),
        decoder.height(),
        decoder.format().descriptor().unwrap().name(),
        stream.time_base(),
        match sar.numerator() {
            0 => "1".to_string(),
            _ => format!("{}/{}", sar.numerator(), sar.denominator()),
        }
    );
    filter.add(&ffmpeg::filter::find("buffer").unwrap(), "in", &buffer_args)?;
    filter.add(&filter::find("buffersink").unwrap(), "out", "")?;
    filter
        .get("out")
        .unwrap()
        .set_pixel_format(PixelFormat::RGB24);
    let drawtext_args = vec![
        "x=(w-tw)/1.05".to_string(),
        "y=h-(2*lh)".to_string(),
        "fontcolor=white".to_string(),
        "fontsize=72".to_string(),
        "box=1".to_string(),
        "boxcolor=black".to_string(),
        "boxborderw=5".to_string(),
        "text=%{pts\\:hms}".to_string(),
    ]
    .join(":");
    filter.output("in", 0)?.input("out", 0)?.parse(
        &vec![
            format!("drawtext='{}'", drawtext_args),
            format!("scale=w={}:h={}", out_width, out_height),
        ]
        .join(","),
    )?;
    filter.validate()?;
    Ok(filter)
}

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
    pub capture_times: Vec<i64>,
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
        let duration = stream.duration();
        let start_at = (duration as f64 * opts.skip) as i64;
        let interval = ((duration - start_at) as f64 / opts.num_captures() as f64) as i64;
        let capture_times: Vec<i64> = repeat(true)
            .take(opts.num_captures() as usize)
            .enumerate()
            .map(|(i, _)| i as i64 * interval + start_at)
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

    pub fn get_frame_at(&mut self, ts: i64, out_width: u32, out_height: u32) -> Result<Vec<u8>> {
        log::debug!("Getting frame at {}", ts);
        let mut decoder = self.create_decoder()?;
        self.input
            .seek_to_frame(self.video_stream_idx as i32, ts, SeekFlags::ANY)?;
        let mut filter =
            create_timestamp_filter(&decoder, &self.stream()?, self.width, self.height)?;
        let mut frame = Video::new(self.pixel_format, out_width, out_height);
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
        filter.get("in").unwrap().source().add(&frame)?;
        let mut rgb_frame = Video::new(PixelFormat::RGB24, out_width, out_height);
        filter.get("out").unwrap().sink().frame(&mut rgb_frame)?;
        Ok(rgb_frame.data(0).to_vec())
    }
}
