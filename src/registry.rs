use crate::plate_detection::bounding_box_render::BoundingBoxRender;
use crate::plate_detection::car_detector::CarDetector;
use crate::plate_detection::dnn_ocr::DnnOcrReader;
use crate::plate_detection::video_reader::VideoReader;
use rusted_pipe::graph::new_node;
use rusted_pipe::{graph::formatter::NodeRegistry, packet::WorkQueue};

pub fn examples_registry() -> NodeRegistry {
    let mut nodes = NodeRegistry::default();
    nodes.insert(
        "BoundingBoxRender".to_string(),
        Box::new(|| new_node(BoundingBoxRender::default(), WorkQueue::default())),
    );
    nodes.insert(
        "CarDetector".to_string(),
        Box::new(|| new_node(CarDetector::default(), WorkQueue::default())),
    );
    nodes.insert(
        "DnnOcrReader".to_string(),
        Box::new(|| new_node(DnnOcrReader::default(), WorkQueue::default())),
    );
    nodes.insert(
        "VideoReader".to_string(),
        Box::new(|| new_node(VideoReader::default(), WorkQueue::default())),
    );
    nodes
}
