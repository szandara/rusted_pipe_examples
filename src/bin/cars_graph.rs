use crossbeam::channel::Receiver;
use rusted_pipe::graph::Graph;

use rusted_pipe::graph::formatter::DotFormatter;
use rusted_pipe::graph::formatter::GraphFormatter;
use rusted_pipe_examples::plate_detection::video_reader::VideoReader;
use rusted_pipe_examples::registry::examples_registry;

fn setup_test() -> (Graph, Receiver<bool>) {
    let graph_formatter = DotFormatter::default();

    let registry = examples_registry();
    let graph = graph_formatter
        .from_file("graphs/cars.dot", registry)
        .unwrap();
    let done_channel_arc = graph.nodes().get("VideoReader").unwrap().handler.clone();
    let done_channel = done_channel_arc.lock().unwrap();
    let done_channel = done_channel
        .downcast_ref::<VideoReader>()
        .unwrap()
        .get_done_event();

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
