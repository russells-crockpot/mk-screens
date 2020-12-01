use std::{iter::repeat, path::PathBuf};

use anyhow::{Error as AnyhowError, Result};
use derivative::Derivative;
use ffmpeg::{
    decoder::Video as VideoDecoder,
    filter::{self, Graph},
    format::{context::Input, stream::Stream, Pixel as PixelFormat},
    media::Type as MediaType,
    util::frame::video::Video,
    Rational,
};

use crate::{
    ffmpeg_ext::{FrameSeekable as _, HasCodedDimensions as _, HasDimensions as _, SeekFlags},
    files::img_file_name,
    opts::Opts,
    util::Dimensions,
    Error,
};

fn format_rational(rational: &Rational) -> String {
    match rational.numerator() {
        0 => "1".to_string(),
        _ => format!("{}/{}", rational.numerator(), rational.denominator()),
    }
}

fn create_timestamp_filter(decoder: &VideoDecoder, stream: &Stream) -> Result<Graph> {
    let mut filter = Graph::new();
    let mut buffer_args = vec![
        format!("width={}", decoder.width()),
        format!("height={}", decoder.height()),
        format!("video_size={}x{}", decoder.width(), decoder.height()),
        format!("time_base={}", stream.time_base()),
        format!("sar={}", format_rational(&decoder.aspect_ratio())),
    ];
    if let Some(frame_rate) = &decoder.frame_rate() {
        buffer_args.push(format!("frame_rate={}", format_rational(frame_rate)));
    }
    if let Some(desc) = decoder.format().descriptor() {
        buffer_args.push(format!("pix_fmt={}", desc.name()));
    }
    filter.add(
        &ffmpeg::filter::find("buffer").unwrap(),
        "in",
        &buffer_args.join(":"),
    )?;
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
    filter
        .output("in", 0)?
        .input("out", 0)?
        .parse(&format!("drawtext='{}'", drawtext_args))?;
    filter.validate()?;
    Ok(filter)
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct VidInfo {
    pub path: PathBuf,
    pub duration: i64,
    pub pixel_format: PixelFormat,
    pub height: u32,
    pub width: u32,
    pub interval: i64,
    pub video_stream_idx: usize,
    #[derivative(Debug = "ignore")]
    pub input: Input,
    #[derivative(Debug = "ignore")]
    pub capture_times: Vec<i64>,
    #[derivative(Debug = "ignore")]
    filter: Graph,
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
        let filter = create_timestamp_filter(&video, &stream)?;
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
            interval: stream.frames() / opts.num_captures() as i64,
            input,
            capture_times,
            filter,
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

    fn get_actual_size(&self, decoder: &VideoDecoder, data_len: u32) -> Dimensions {
        let pixels = data_len / 3;
        let coded_dimensions = decoder.coded_dimensions();
        let video_dimensions = decoder.dimensions();
        if coded_dimensions == video_dimensions {
            video_dimensions
        } else if coded_dimensions.area() == pixels {
            coded_dimensions
        } else {
            video_dimensions
        }
    }

    pub fn get_frame_at(&mut self, ts: i64) -> Result<(Dimensions, Vec<u8>)> {
        let mut decoder = self.create_decoder()?;
        self.input
            .seek_to_frame(self.video_stream_idx as i32, ts, SeekFlags::ANY)?;
        let mut frame = Video::empty();
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
        self.filter.get("in").unwrap().source().add(&frame)?;
        let mut rgb_frame = Video::empty();
        self.filter
            .get("out")
            .unwrap()
            .sink()
            .frame(&mut rgb_frame)?;
        let data = rgb_frame.data(0).to_vec();
        Ok((self.get_actual_size(&decoder, data.len() as u32), data))
    }
}
