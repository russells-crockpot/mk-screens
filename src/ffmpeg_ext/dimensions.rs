use crate::util::Dimensions;
use ffmpeg::{decoder::Video as VideoDecoder, util::frame::video::Video};

pub trait HasDimensions {
    fn dimensions(&self) -> Dimensions;
}

pub trait HasWidthAndHeight: HasDimensions {
    fn width(&self) -> u32;
    fn height(&self) -> u32;
    fn dimensions(&self) -> Dimensions {
        Dimensions(self.width(), self.height())
    }
}

impl HasDimensions for VideoDecoder {
    fn dimensions(&self) -> Dimensions {
        Dimensions::new(self.width(), self.height())
    }
}

impl HasDimensions for Video {
    fn dimensions(&self) -> Dimensions {
        Dimensions::new(self.width(), self.height())
    }
}
