use crossbeam::channel::Receiver;
use rusted_pipe::graph::Graph;

use rusted_pipe::graph::formatter::DotFormatter;
use rusted_pipe::graph::formatter::GraphFormatter;
use rusted_pipe_examples::registry::examples_registry;

fn setup_test() -> Graph {
    let graph_formatter = DotFormatter::default();

    let registry = examples_registry();
    let graph = graph_formatter
        .from_file("graphs/cars.dot", registry)
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
