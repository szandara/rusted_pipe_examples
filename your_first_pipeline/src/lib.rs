pub mod utils;

use rusted_pipe::{
    channels::{
        read_channel::InputGenerator, typed_read_channel::ReadChannel2,
        typed_write_channel::WriteChannel1,
    },
    graph::processor::{ProcessorWriter, SourceProcessor, TerminalProcessor},
    DataVersion, RustedPipeError,
};
use utils::FpsLimiter;

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
