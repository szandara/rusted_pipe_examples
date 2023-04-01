use std::{thread, time::Duration};

use opencv::{
    core::{Rect, Vector},
    prelude::Mat,
};
use rusted_pipe::{
    buffers::synchronizers::timestamp::TimestampSynchronizer,
    channels::{
        typed_read_channel::{NoBuffer, ReadChannel1, ReadChannel3},
        typed_write_channel::WriteChannel1,
    },
    graph::{
        graph::Graph,
        processor::{Node, Nodes, SourceNode, TerminalNode},
    },
};
use rusted_pipe_examples::plate_detection::dnn_ocr::DnnOcrReader;
use rusted_pipe_examples::plate_detection::video_reader::VideoReader;
use rusted_pipe_examples::plate_detection::CarWithText;
use rusted_pipe_examples::plate_detection::{
    bounding_box_render::BoundingBoxRender, car_detector::CarDetector,
};

fn setup_test() -> Graph {
    let car_image_channel =
        ReadChannel1::<Mat>::setup_read_channel(2000, true, TimestampSynchronizer::default());

    let ocr_image_channel =
        ReadChannel1::<Mat>::setup_read_channel(2000, true, TimestampSynchronizer::default());

    let bounding_box_channel =
        ReadChannel3::<Vector<Rect>, Vec<CarWithText>, Mat>::setup_read_channel(
            2000,
            true,
            TimestampSynchronizer::default(),
        );

    let mut image_writer = WriteChannel1::<Mat>::create();
    let mut bbox_writer = WriteChannel1::<Vector<Rect>>::create();
    let mut car_with_text_writer = WriteChannel1::<Vec<CarWithText>>::create();

    // Frame -> OCR
    rusted_pipe::graph::graph::link(
        ocr_image_channel.channels.lock().unwrap().c1(),
        image_writer.c1(),
    )
    .unwrap();

    // Frame -> Car Detecctor
    rusted_pipe::graph::graph::link(
        car_image_channel.channels.lock().unwrap().c1(),
        image_writer.c1(),
    )
    .unwrap();

    // Frame -> BoundingBox
    rusted_pipe::graph::graph::link(
        bounding_box_channel.channels.lock().unwrap().c3(),
        image_writer.c1(),
    )
    .unwrap();

    rusted_pipe::graph::graph::link(
        bounding_box_channel.channels.lock().unwrap().c1(),
        bbox_writer.c1(),
    )
    .unwrap();

    rusted_pipe::graph::graph::link(
        bounding_box_channel.channels.lock().unwrap().c2(),
        car_with_text_writer.c1(),
    )
    .unwrap();

    let video_input = VideoReader::default();
    let video_input_node = SourceNode::create(
        "video_input".to_string(),
        Box::new(video_input),
        image_writer,
    );

    let car_detector = CarDetector::default();
    let car_detector_node = Node::create(
        "car_detector".to_string(),
        Box::new(car_detector),
        car_image_channel,
        bbox_writer,
    );

    let ocr_detector = DnnOcrReader::default();
    let ocr_detector_node = Node::create(
        "ocr_detector".to_string(),
        Box::new(ocr_detector),
        ocr_image_channel,
        car_with_text_writer,
    );

    let bbox_render = BoundingBoxRender::default();
    let bbox_reader_node = TerminalNode::create(
        "bbox_render".to_string(),
        Box::new(bbox_render),
        bounding_box_channel,
    );

    let mut graph = Graph::new();
    graph.start_node(Nodes::NodeHandler(Box::new(ocr_detector_node)));
    graph.start_node::<ReadChannel3<Vector<Rect>, Vec<CarWithText>, Mat>, WriteChannel1<String>>(
        Nodes::TerminalHandler(Box::new(bbox_reader_node)),
    );
    graph.start_node(Nodes::NodeHandler(Box::new(car_detector_node)));
    graph.start_node::<NoBuffer, WriteChannel1<Mat>>(Nodes::SourceHandler(Box::new(
        video_input_node,
    )));

    graph
}

fn main() {
    let mut graph = setup_test();

    println!("Starting, waiting for video to end");
    thread::sleep(Duration::from_millis(4000));
    graph.stop(true, None);
    println!("Done");
}
