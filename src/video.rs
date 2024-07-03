//! Items relating to video files.

use std::{
    iter::repeat,
    path::{Path, PathBuf},
};

use derivative::Derivative;
use ffmpeg::{
    codec::context::Context as CodecContext,
    decoder::Video as VideoDecoder,
    filter::{self, Graph},
    format::{context::Input, stream::Stream, Pixel as PixelFormat},
    util::{dictionary::Owned as FfmpegDictionary, frame::video::Video, media::Type as MediaType},
    Rational,
};

use crate::{
    ffmpeg_ext::LinkableGraph as _,
    files::img_file_name,
    settings::Settings,
    util::{Dimensions, ENV},
    Error, Result,
};

const BACK_TRIM_AMOUNT: f64 = 0.01;

#[inline]
fn input_opts<'a>() -> FfmpegDictionary<'a> {
    let mut opts = FfmpegDictionary::new();
    opts.set("probesize", ENV.ffmpeg_probesize());
    opts.set("analyzeduration", ENV.ffmpeg_analyzeduration());
    opts
}

fn format_rational(rational: &Rational) -> String {
    match rational.numerator() {
        0 => "1".to_string(),
        _ => format!("{}/{}", rational.numerator(), rational.denominator()),
    }
}

fn create_filter_graph(
    decoder: &VideoDecoder,
    stream: &Stream,
    out_dims: &Dimensions,
) -> Result<Graph> {
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
    let drawtext_args = [
        "x=(w-tw)/1.05".to_string(),
        "y=h-(2*lh)".to_string(),
        "fontcolor=white".to_string(),
        format!("fontsize={}", out_dims.height() / 7),
        "box=1".to_string(),
        "boxcolor=black".to_string(),
        format!("boxborderw={}", out_dims.height() / 45),
        "text=%{pts\\:hms}".to_string(),
    ]
    .join(":");
    graph.add(
        &filter::find("format").unwrap(),
        "pix_fmt",
        PixelFormat::RGB24.descriptor().unwrap().name(),
    )?;
    graph.add(&filter::find("drawtext").unwrap(), "btc", &drawtext_args)?;
    graph.add(
        &filter::find("scale").unwrap(),
        "scale",
        &[
            format!("w={}", out_dims.width()),
            format!("h={}", out_dims.height()),
            "eval=frame".to_string(),
            "flags=fast_bilinear".to_string(),
        ]
        .join(":"),
    )?;
    graph.chain_link(&["in", "pix_fmt", "scale", "btc", "out"])?;
    graph.validate()?;
    Ok(graph)
}

pub fn find_best_stream<P: AsRef<Path>>(input: &Input, path: P) -> Result<Stream> {
    input
        .streams()
        .find(|s| s.parameters().medium() == MediaType::Video)
        .ok_or_else(|| Error::NoVideoStream {
            path: path.as_ref().to_path_buf(),
        })
}

#[derive(Derivative)]
#[derivative(Debug)]
/// Contains relevant information about a video file.
pub struct VidInfo {
    path: PathBuf,
    duration: i64,
    pixel_format: PixelFormat,
    dimensions: Dimensions,
    capture_dimensions: Dimensions,
    interval: i64,
    video_stream_idx: usize,
    #[derivative(Debug = "ignore")]
    input: Input,
    #[derivative(Debug = "ignore")]
    filter: Graph,
}

impl VidInfo {
    pub fn new<P: AsRef<Path>>(settings: &Settings, path: P) -> Result<Self> {
        let input = ffmpeg::format::input_with_dictionary(&path, input_opts())?;
        let stream = find_best_stream(&input, &path)?;
        let decoder = CodecContext::from_parameters(stream.parameters())?
            .decoder()
            .video()?;
        let dimensions = Dimensions::new(decoder.width(), decoder.height());
        let mut capture_width = (settings.width() - (settings.columns() * 4)) / settings.columns();
        if !settings.scale_up() && capture_width > dimensions.width() {
            capture_width = dimensions.width();
        }
        let capture_height =
            (capture_width as f64 / dimensions.width() as f64) * dimensions.height() as f64;
        let capture_dimensions = Dimensions::new(capture_width, capture_height as u32);
        let filter = create_filter_graph(&decoder, &stream, &capture_dimensions)?;
        let pixel_format = decoder.format();
        Ok(Self {
            path: path.as_ref().to_path_buf(),
            duration: input.duration(),
            pixel_format,
            dimensions,
            capture_dimensions,
            video_stream_idx: stream.index(),
            interval: stream.frames() / settings.num_captures() as i64,
            input,
            filter,
        })
    }

    /// Generates a list of timestamps where individual frames should be captured.
    pub fn generate_capture_times(&self, settings: &Settings) -> Vec<i64> {
        let start_at = (self.duration as f64 * settings.skip()) as i64;
        let back_trim = (self.duration as f64 * BACK_TRIM_AMOUNT) as i64;
        let interval =
            ((self.duration - start_at - back_trim) as f64 / settings.num_captures() as f64) as i64;
        repeat(true)
            .take(settings.num_captures() as usize)
            .enumerate()
            .map(|(i, _)| i as i64 * interval + start_at)
            .collect()
    }

    /// The path to the original video file.
    pub fn path(&self) -> &Path {
        self.path.as_ref()
    }

    /// The pixel format of the original video file.
    pub fn pixel_format(&self) -> PixelFormat {
        self.pixel_format
    }

    pub fn capture_dimensions(&self) -> &Dimensions {
        &self.capture_dimensions
    }

    pub fn width(&self) -> u32 {
        self.dimensions.width()
    }

    pub fn height(&self) -> u32 {
        self.dimensions.height()
    }

    /// Returns the video stream for the underlying video file.
    pub fn stream(&self) -> Result<Stream<'_>> {
        find_best_stream(&self.input, &self.path)
    }

    pub fn img_file_name(&self) -> String {
        img_file_name(&self.path)
    }

    fn create_decoder(&self) -> Result<VideoDecoder> {
        Ok(CodecContext::from_parameters(self.stream()?.parameters())?
            .decoder()
            .video()?)
    }

    fn get_actual_size(&self, frame: &Video) -> Dimensions {
        Dimensions((frame.stride(0) / 3) as u32, frame.height())
    }

    /// Gets the frame image at (or near) the provided timestamp.
    pub fn get_frame_at(&mut self, timestamp: i64) -> Result<(Dimensions, Vec<u8>)> {
        let mut decoder = self.create_decoder()?;
        self.input.seek(timestamp, timestamp..self.duration)?;
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
                if decoder.send_packet(packet).is_err() {
                    return true;
                }
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
