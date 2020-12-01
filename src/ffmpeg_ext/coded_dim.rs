use crate::util::Dimensions;
use ffmpeg::codec::context::Context;

pub trait HasCodedDimensions {
    fn coded_width(&self) -> u32;
    fn coded_height(&self) -> u32;
    fn coded_dimensions(&self) -> Dimensions;
}

impl HasCodedDimensions for Context {
    fn coded_width(&self) -> u32 {
        unsafe { (*self.as_ptr()).coded_width as u32 }
    }

    fn coded_height(&self) -> u32 {
        unsafe { (*self.as_ptr()).coded_height as u32 }
    }

    fn coded_dimensions(&self) -> Dimensions {
        Dimensions::new(self.coded_width(), self.coded_height())
    }
}
