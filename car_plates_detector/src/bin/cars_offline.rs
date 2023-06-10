use std::{thread, time::Duration};

use car_plates_detector::plate_detection::dnn_ocr::DnnOcrReader;
use car_plates_detector::plate_detection::video_reader::VideoReader;
use car_plates_detector::plate_detection::{
    bounding_box_render::BoundingBoxRender, object_detector::ObjectDetector,
};
use rusted_pipe::graph::metrics::Metrics;
use rusted_pipe::{
    buffers::synchronizers::timestamp::TimestampSynchronizer,
    graph::{
        build::Graph,
        metrics::default_prometheus_address,
        processor::{Node, SourceNode},
    },
};

fn setup_test(metrics: Metrics) -> Graph {
    // Create the nodes
    // Node that reads the data from the input file
    let mut video_input_node = SourceNode::create_common(
        "video_input".to_string(),
        Box::new(VideoReader::default(false, 5)),
    );

    let timestamp_synch = TimestampSynchronizer::default();

    // Node that performs bounding box detection for cars
    let mut car_detector_node = Node::create_common(
        "car_detector".to_string(),
        Box::new(ObjectDetector::car_detector(true)),
        true,
        3000,
        3000,
        Box::new(timestamp_synch.clone()),
        true,
    );

    // Node that performs bounding box detection for cars
    let mut plate_detector_node = Node::create_common(
        "plate_detector".to_string(),
        Box::new(ObjectDetector::plate_detector(true)),
        true,
        3000,
        3000,
        Box::new(timestamp_synch.clone()),
        true,
    );

    // Node that performs OCR detection on images.
    let mut ocr_detector_node = Node::create_common(
        "ocr_detector".to_string(),
        Box::new(DnnOcrReader::default()),
        true,
        3000,
        3000,
        Box::new(timestamp_synch.clone()),
        true,
    );

    // Node that collects the inferred information and overlays it on top of the original video.
    let bbox_render_node = Node::create_common(
        "bbox_render".to_string(),
        Box::new(BoundingBoxRender::with_save_to_file()),
        true,
        5000,
        5000,
        Box::new(timestamp_synch.clone()),
        true,
    );

    // Link nodes together to form a graph.

    // Each node with a write channel can be linked to a read channel of another node.
    // The compiler will make sure that the data types are compatible.

    // Write channels are 1 to many. So data written on an output channel can be fan out
    // to different consumers.

    // Read channels are 1 to 1. Data incoming into a read channel is always from the same source.

    // Frame -> OCR
    rusted_pipe::graph::build::link(
        video_input_node.write_channel.writer.c1(),
        ocr_detector_node
            .read_channel
            .channels
            .write()
            .unwrap()
            .c1(),
    )
    .unwrap();

    // Frame -> Car Detector
    rusted_pipe::graph::build::link(
        video_input_node.write_channel.writer.c1(),
        car_detector_node
            .read_channel
            .channels
            .write()
            .unwrap()
            .c1(),
    )
    .unwrap();

    // Frame -> Plate Detector
    rusted_pipe::graph::build::link(
        video_input_node.write_channel.writer.c1(),
        plate_detector_node
            .read_channel
            .channels
            .write()
            .unwrap()
            .c1(),
    )
    .unwrap();

    // Plate Detector -> OCR
    rusted_pipe::graph::build::link(
        plate_detector_node.write_channel.writer.c1(),
        ocr_detector_node
            .read_channel
            .channels
            .write()
            .unwrap()
            .c2(),
    )
    .unwrap();

    // Frame -> BoundingBox
    rusted_pipe::graph::build::link(
        video_input_node.write_channel.writer.c1(),
        bbox_render_node.read_channel.channels.write().unwrap().c3(),
    )
    .unwrap();

    // Car Detector -> BoundingBox
    rusted_pipe::graph::build::link(
        car_detector_node.write_channel.writer.c1(),
        bbox_render_node.read_channel.channels.write().unwrap().c1(),
    )
    .unwrap();

    // OCR -> BoundingBox
    rusted_pipe::graph::build::link(
        ocr_detector_node.write_channel.writer.c1(),
        bbox_render_node.read_channel.channels.write().unwrap().c2(),
    )
    .unwrap();

    // Create the graph objects and start the graph scheduler
    let mut graph = Graph::new(metrics);

    // We need to start each node independently
    graph.start_node(ocr_detector_node);
    graph.start_node(plate_detector_node);
    graph.start_node(bbox_render_node);
    graph.start_node(car_detector_node);
    graph.start_source_node(video_input_node);

    graph
}

fn main() {
    let metrics = Metrics::builder().with_prometheus(&default_prometheus_address());
    let graph = setup_test(metrics);

    println!("Starting, waiting for video to end");
    thread::sleep(Duration::from_millis(15000));
    graph.stop(true, None);

    println!("Done");
}
