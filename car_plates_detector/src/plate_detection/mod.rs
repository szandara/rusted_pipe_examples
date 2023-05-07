pub mod bounding_box_render;
pub mod dnn_ocr;
pub mod object_detector;
pub mod rtp_sink;
pub mod video_reader;

use opencv::core::Rect;

#[derive(Clone)]
pub struct CarWithText {
    plate: Option<String>,
    car: Rect,
}

impl CarWithText {
    fn new(plate: Option<String>, car: Rect) -> Self {
        return Self { plate, car };
    }
}
