use std::{iter::repeat, path::PathBuf};

use anyhow::{Error as AnyhowError, Result};
use derivative::Derivative;
use ffmpeg::{
    decoder::Video as VideoDecoder,
    filter::{self, Graph},
    format::{context::Input, stream::Stream, Pixel as PixelFormat},
    media::Type as MediaType,
    software::{
        scaler,
        scaling::{context::Context as ScalingContext, flag::Flags as ScalingFlags},
    },
    util::frame::video::Video,
    Rational,
};

use crate::{
    ffmpeg_ext::{FrameSeekable as _, LinkableGraph as _, SeekFlags},
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

fn create_filter_graph(decoder: &VideoDecoder, stream: &Stream, out_width: u32) -> Result<Graph> {
    let mut graph = Graph::new();
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
    graph.add(
        &ffmpeg::filter::find("buffer").unwrap(),
        "in",
        &buffer_args.join(":"),
    )?;
    graph.add(&filter::find("buffersink").unwrap(), "out", "")?;
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
    graph.add(
        &filter::find("format").unwrap(),
        "pix_fmt",
        &PixelFormat::RGB24.descriptor().unwrap().name(),
    )?;
    graph.add(&filter::find("drawtext").unwrap(), "btc", &drawtext_args)?;
    graph.add(
        &filter::find("scale").unwrap(),
        "scale",
        &vec![
            format!("w={}", out_width),
            "h=-1".to_string(),
            "eval=frame".to_string(),
            "flags=fast_bilinear".to_string(),
        ]
        .join(":"),
    )?;
    graph.chain_link(&["in", "pix_fmt", "btc", "scale", "out"])?;
    graph.validate()?;
    Ok(graph)
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct VidInfo {
    pub path: PathBuf,
    duration: i64,
    pub pixel_format: PixelFormat,
    dimensions: Dimensions,
    pub capture_dimensions: Dimensions,
    interval: i64,
    video_stream_idx: usize,
    #[derivative(Debug = "ignore")]
    input: Input,
    #[derivative(Debug = "ignore")]
    pub capture_times: Vec<i64>,
    #[derivative(Debug = "ignore")]
    filter: Graph,
    #[derivative(Debug = "ignore")]
    scaler: ScalingContext,
}

impl VidInfo {
    pub fn new(opts: &Opts, path: PathBuf) -> Result<Self> {
        let input = ffmpeg::format::input(&path)?;
        let stream = if let Some(stream) = input.streams().best(MediaType::Video) {
            stream
        } else {
            return Err(AnyhowError::from(Error::NoVideoStream(path)));
        };
        let decoder = match stream.codec().decoder().video() {
            Ok(v) => v,
            Err(e) => return Err(AnyhowError::from(Error::CorruptVideoStream(path, e))),
        };
        let dimensions = Dimensions::new(decoder.width(), decoder.height());
        let duration = stream.duration();
        let start_at = (duration as f64 * opts.skip) as i64;
        let interval = ((duration - start_at) as f64 / opts.num_captures() as f64) as i64;
        let mut capture_width = (opts.width - (opts.columns * 4)) / opts.columns;
        if !opts.scale_up && capture_width > dimensions.width() {
            capture_width = dimensions.width();
        }
        let capture_height =
            (capture_width as f64 / dimensions.width() as f64) * dimensions.height() as f64;
        let capture_dimensions = Dimensions::new(capture_width, capture_height as u32);
        let scaler = scaler(
            PixelFormat::RGB24,
            ScalingFlags::BILINEAR,
            dimensions.as_tuple(),
            capture_dimensions.as_tuple(),
        )?;
        let filter = create_filter_graph(&decoder, &stream, capture_width)?;
        let capture_times: Vec<i64> = repeat(true)
            .take(opts.num_captures() as usize)
            .enumerate()
            .map(|(i, _)| i as i64 * interval + start_at)
            .collect();
        Ok(Self {
            path,
            duration,
            pixel_format: decoder.format(),
            dimensions,
            capture_dimensions,
            video_stream_idx: stream.index(),
            interval: stream.frames() / opts.num_captures() as i64,
            input,
            capture_times,
            filter,
            scaler,
        })
    }

    pub fn width(&self) -> u32 {
        self.dimensions.width()
    }
    pub fn height(&self) -> u32 {
        self.dimensions.height()
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

    fn get_actual_size(&self, frame: &Video) -> Dimensions {
        Dimensions((frame.stride(0) / 3) as u32, frame.height())
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
        Ok((self.get_actual_size(&rgb_frame), data))
    }
}
