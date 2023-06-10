use std::{thread, time::Duration};

use car_plates_detector::plate_detection::dnn_ocr::DnnOcrReader;
use car_plates_detector::plate_detection::video_reader::VideoReader;
use car_plates_detector::plate_detection::{
    bounding_box_render::BoundingBoxRender, object_detector::ObjectDetector, rtp_sink::RtpSink,
};
use rusted_pipe::graph::metrics::{default_prometheus_address, Metrics};
use rusted_pipe::{
    buffers::synchronizers::{real_time::RealTimeSynchronizer, timestamp::TimestampSynchronizer},
    graph::{
        build::Graph,
        processor::{Node, SourceNode, TerminalNode},
    },
};

fn setup_test() -> Graph {
    // Create the nodes

    // Node that reads the data from the input file
    let mut video_input_node = SourceNode::create_common(
        "video_input".to_string(),
        Box::new(VideoReader::default(false, 5)),
    );

    let realtime_synch = RealTimeSynchronizer::new(1e7 as u128, false, true);
    let timestamp_synch = TimestampSynchronizer::default();

    // Node that performs bounding box detection for cars
    let mut car_detector_node = Node::create_common(
        "car_detector".to_string(),
        Box::new(ObjectDetector::car_detector(true)),
        false,
        1,
        1,
        Box::new(timestamp_synch.clone()),
        true,
    );

    // Node that performs OCR detection on images.
    let mut ocr_detector_node = Node::create_common(
        "ocr_detector".to_string(),
        Box::new(DnnOcrReader::default()),
        false,
        50,
        1,
        Box::new(timestamp_synch.clone()),
        true,
    );

    // Node that performs bounding box detection for cars
    let mut plate_detector_node = Node::create_common(
        "plate_detector".to_string(),
        Box::new(ObjectDetector::plate_detector(true)),
        false,
        1,
        1,
        Box::new(timestamp_synch.clone()),
        true,
    );

    // Node that collects the inferred information and overlays it on top of the original video.
    let mut bbox_render_node = Node::create_common(
        "bbox_render".to_string(),
        Box::new(BoundingBoxRender::default()),
        false,
        100,
        1,
        Box::new(realtime_synch.clone()),
        true,
    );

    // Node that collects the inferred information and overlays it on top of the original video.
    let rtp_node = TerminalNode::create_common(
        "rtp".to_string(),
        Box::new(RtpSink::new(2, "127.0.0.1", 5000)),
        false,
        50,
        1,
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

    // BoundingBox -> Rtp
    rusted_pipe::graph::build::link(
        bbox_render_node.write_channel.writer.c1(),
        rtp_node.read_channel.channels.write().unwrap().c1(),
    )
    .unwrap();

    // Create the graph objects and start the graph scheduler
    let metrics = Metrics::builder().with_prometheus(&default_prometheus_address());
    let mut graph = Graph::new(metrics);

    // We need to start each node independently
    graph.start_terminal_node(rtp_node);
    graph.start_node(bbox_render_node);
    graph.start_node(car_detector_node);
    graph.start_node(plate_detector_node);
    graph.start_node(ocr_detector_node);
    graph.start_source_node(video_input_node);

    graph
}

fn main() {
    let graph = setup_test();

    println!("Starting, waiting for video to end");
    thread::sleep(Duration::from_millis(4000));
    graph.stop(true, None);
    println!("Done");
}
