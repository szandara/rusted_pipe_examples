use std::{thread, time::Duration};

use rusted_pipe::{
    buffers::synchronizers::real_time::RealTimeSynchronizer,
    graph::{
        metrics::Metrics,
        processor::{SourceNode, TerminalNode},
        build::link,
        build::Graph
    },
};
use your_first_pipeline::{Consumer, Producer};


fn setup_test() -> Graph {
    // Create the nodes

    // Node that reads the data from the input file
    let mut slow_producer =
        SourceNode::create_common("slow".to_string(), Box::new(Producer::new(10)));
    let mut fast_producer =
        SourceNode::create_common("fast".to_string(), Box::new(Producer::new(100)));

    // Synch with 1ms tolerance
    let consumer_synch = RealTimeSynchronizer::new(1e6 as u128, true, false);
    let consumer = TerminalNode::create_common(
        "consumer".to_string(),
        Box::new(Consumer::default()),
        true,
        1000,
        1000,
        Box::new(consumer_synch), true
    );

    // Create the graph objects and start the graph scheduler
    let mut graph = Graph::new(Metrics::no_metrics());

    link(
        slow_producer.write_channel.writer.c1(),
        consumer.read_channel.channels.write().unwrap().c1(),
    )
    .unwrap();

    link(
        fast_producer.write_channel.writer.c1(),
        consumer.read_channel.channels.write().unwrap().c2(),
    )
    .unwrap();

    graph.start_source_node(slow_producer);
    graph.start_source_node(fast_producer);
    graph.start_terminal_node(consumer);

    graph
}

fn main() {
    let graph = setup_test();
    thread::sleep(Duration::from_secs(10));
    graph.stop(false, None);
    println!("Done");
}
