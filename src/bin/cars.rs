use crossbeam::channel::Receiver;
use rusted_pipe::graph::{Graph, Node};
use rusted_pipe::packet::{ChannelID, WorkQueue};

use rusted_pipe_examples::plate_detection::bounding_box_render::BoundinBoxRender;
use rusted_pipe_examples::plate_detection::car_detector::CarDetector;
use rusted_pipe_examples::plate_detection::dnn_ocr::DnnOcrReader;
use rusted_pipe_examples::plate_detection::video_reader::VideoReader;

use std::sync::{Arc, Mutex};

fn setup_test() -> (Graph, Receiver<bool>) {
    let processor = VideoReader::default();
    let done_channel = processor.get_done_event();
    let image_input = Node::default(Arc::new(Mutex::new(processor)), WorkQueue::default(), true);

    let processor = CarDetector::default();
    let detector = Node::default(Arc::new(Mutex::new(processor)), WorkQueue::new(2000), false);

    let processor = BoundinBoxRender::default();
    let boundingbox = Node::default(Arc::new(Mutex::new(processor)), WorkQueue::new(2000), false);

    let processor = DnnOcrReader::default();
    let ocr = Node::default(Arc::new(Mutex::new(processor)), WorkQueue::new(2000), false);

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
            &"BoundinBoxRender".to_string(),
            &ChannelID::from("cars"),
        )
        .unwrap();

    graph
        .link(
            &"VideoReader".to_string(),
            &ChannelID::from("image"),
            &"BoundinBoxRender".to_string(),
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
            &"BoundinBoxRender".to_string(),
            &ChannelID::from("plates"),
        )
        .unwrap();
    return (graph, done_channel);
}

fn main() {
    let (mut graph, done_channel) = setup_test();

    graph.start();
    println!("Starting, waiting for video to end");
    done_channel.recv().unwrap();
    graph.stop();
    println!("Done");
}
