
## Your first pipeline

Let's create a simple producer that generates some data at a given fps. `FpsLimiter` can be found in the `rusted_pipe_examples` repo. Since this is a SourceProcessor, `handle` is called by the scheduler as soon as they are ready. ie. they are not processing anything. In real life this data would come from a sensor at a given sensor frame rate. Here we simulate different producing speed. The producer generates a string.

```
pub struct Producer {
    fps_limiter: FpsLimiter,
}

impl Producer {
    pub fn new(fps: usize) -> Self {
        let fps = fps;
        let fps_limiter = FpsLimiter::new(fps);
        Self { fps_limiter }
    }
}

impl SourceProcessor for Producer {
    type OUTPUT = WriteChannel1<String>;
    fn handle(&mut self, mut output: ProcessorWriter<Self::OUTPUT>) -> Result<(), RustedPipeError> {
        let frame_ts = DataVersion::from_now();
        output
            .writer
            .c1()
            .write("I have data!".to_string(), &frame_ts)
            .unwrap();

        self.fps_limiter.wait();

        Ok(())
    }
}
```

Now let's create another processor that uses data from two producers. The data is expected to be synchronized.
Each time the sychronizer receives a payload it calls `handle`. The method can access the synchronized payload (two channels in this case). Data in each channel is expected to be there but this is a choice of the Processor itself. Processors can work with missing channel data. In this case we throw an error if data is missing.

```

#[derive(Default)]
pub struct Consumer {}

impl TerminalProcessor for Consumer {
    type INPUT = ReadChannel2<String, String>;
    fn handle(
        &mut self,
        mut input: <Self::INPUT as InputGenerator>::INPUT,
    ) -> Result<(), RustedPipeError> {
        let s1 = input.c1_owned();
        let s2 = input.c2_owned();

        if let (Some(s1), Some(s2)) = (s1, s2) {
            println!(
                "Received {} from s1 at {} and {} from s2 at {}",
                s1.data, s1.version.timestamp_ns, s2.data, s2.version.timestamp_ns
            )
        } else {
            eprintln!("Error channels are not synced");
        }

        Ok(())
    }
}

```

Finally let's create our graph and run it. First instantiate the two producers producing speed at 10 and 100 fps respectively.
```
let mut slow_producer =
    SourceNode::create_common("slow".to_string(), Box::new(Producer::new(10)));
let mut fast_producer =
    SourceNode::create_common("fast".to_string(), Box::new(Producer::new(100)));
```

Then create the consumer with a given sycnrhonization strategy. In this case we want 1 ms tolerance without buffering and we also set `wait_all` in RealTimeSynchronizer to make sure that the processor is called only if both data channels have data.

```
// Synch with 1ms tolerance
let consumer_synch = RealTimeSynchronizer::new(1e6 as u128, true, false);
let consumer = TerminalNode::create_common(
    "consumer".to_string(),
    Box::new(Consumer::default()),
    true,
    1000,
    1000,
    SynchronizerTypes::REALTIME(consumer_synch),
);
```

Link the processors together
```
rusted_pipe::graph::graph::link(
    slow_producer.write_channel.writer.c1(),
    consumer.read_channel.channels.lock().unwrap().c1(),
)
.unwrap();

rusted_pipe::graph::graph::link(
    fast_producer.write_channel.writer.c1(),
    consumer.read_channel.channels.lock().unwrap().c2(),
)
.unwrap();
```

and finally create the graph and run it.

```
let mut graph = Graph::new();

graph.start_source_node(slow_producer);
graph.start_source_node(fast_producer);
graph.start_terminal_node(consumer);

thread::sleep(Duration::from_secs(10));
graph.stop(false, None);
```

The graph will run for 10 seconds and produce some output, then kill its nodes.

