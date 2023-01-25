use rusted_pipe::graph::{Graph, Node};
use rusted_pipe::packet::{ChannelID, WorkQueue};

use rusted_pipe_examples::plate_detection::bounding_box_render::BoundingBoxRender;
use rusted_pipe_examples::plate_detection::car_detector::CarDetector;
use rusted_pipe_examples::plate_detection::dnn_ocr::DnnOcrReader;
use rusted_pipe_examples::plate_detection::video_reader::VideoReader;

use std::sync::{Arc, Mutex};

fn setup_test() -> Graph {
    let processor = VideoReader::default();
    let image_input = Node::default(Arc::new(Mutex::new(processor)), WorkQueue::default());

    let processor = CarDetector::default();
    let detector = Node::default(Arc::new(Mutex::new(processor)), WorkQueue::new(2000));

    let processor = BoundingBoxRender::default();
    let boundingbox = Node::default(Arc::new(Mutex::new(processor)), WorkQueue::new(2000));

    let processor = DnnOcrReader::default();
    let ocr = Node::default(Arc::new(Mutex::new(processor)), WorkQueue::new(2000));

    let mut graph = Graph::new();

    graph.add_node(image_input).unwrap();
    graph.add_node(detector).unwrap();
    graph.add_node(boundingbox).unwrap();
    graph.add_node(ocr).unwrap();

    graph
        .link(
            &"VideoReader".to_string(),
            &ChannelID::from("image"),
            &"CarDetector".to_string(),
            &ChannelID::from("image"),
        )
        .unwrap();
    graph
        .link(
            &"CarDetector".to_string(),
            &ChannelID::from("cars"),
            &"BoundingBoxRender".to_string(),
            &ChannelID::from("cars"),
        )
        .unwrap();

    graph
        .link(
            &"VideoReader".to_string(),
            &ChannelID::from("image"),
            &"BoundingBoxRender".to_string(),
            &ChannelID::from("image"),
        )
        .unwrap();

    graph
        .link(
            &"VideoReader".to_string(),
            &ChannelID::from("image"),
            &"DnnOcrReader".to_string(),
            &ChannelID::from("image"),
        )
        .unwrap();

    graph
        .link(
            &"DnnOcrReader".to_string(),
            &ChannelID::from("plates"),
            &"BoundingBoxRender".to_string(),
            &ChannelID::from("plates"),
        )
        .unwrap();
    graph
}

fn main() {
    let mut graph = setup_test();

    graph.start();
    println!("Starting, waiting for video to end");
    graph.stop(true);
    println!("Done");
}
