use opencv::core::Size;

use opencv::imgproc::resize;
use opencv::imgproc::INTER_LINEAR;
use opencv::prelude::Mat;
use opencv::prelude::MatTraitConst;

use opencv::prelude::VideoCaptureTrait;
use opencv::videoio::VideoCapture;

use opencv::videoio::CAP_ANY;
use rusted_pipe::channels::typed_write_channel::WriteChannel1;
use rusted_pipe::graph::processor::ProcessorWriter;
use rusted_pipe::graph::processor::SourceProcessor;
use rusted_pipe::DataVersion;
use rusted_pipe::RustedPipeError;

use std::thread;
use std::time::Duration;
use std::time::Instant;

pub struct VideoReader {
    capture: VideoCapture,
    fps_control: Instant,
    fps_wait: Duration,
    _fps: u64,
    do_loop: bool,
}

fn make_video() -> VideoCapture {
    VideoCapture::from_file("data/210112_01_Covid Oxford_4k_061.mp4", CAP_ANY).unwrap()
}
impl VideoReader {
    pub fn default(do_loop: bool) -> Self {
        let fps = 20;
        Self {
            capture: make_video(),
            fps_control: Instant::now(),
            fps_wait: Duration::from_millis(1000 / fps),
            _fps: fps,
            do_loop,
        }
    }
}

impl SourceProcessor for VideoReader {
    type OUTPUT = WriteChannel1<Mat>;
    fn handle(&mut self, mut output: ProcessorWriter<Self::OUTPUT>) -> Result<(), RustedPipeError> {
        let mut image = Mat::default();
        let grabbed = self.capture.read(&mut image).unwrap();

        if !grabbed || image.empty() {
            if self.do_loop {
                self.capture = make_video();
                self.capture.read(&mut image).unwrap();
            } else {
                return Err(RustedPipeError::EndOfStream());
            }
        }

        let mut image_resized = Mat::default();
        resize(
            &image,
            &mut image_resized,
            Size::new(640, 480),
            0.0,
            0.0,
            INTER_LINEAR,
        )
        .unwrap();
        let frame_ts = DataVersion::from_now();
        println!("Frame {}", frame_ts.timestamp_ns);
        output.writer.c1().write(image_resized, &frame_ts).unwrap();
        let elapsed = self.fps_control.elapsed();

        if self.fps_wait > elapsed {
            thread::sleep(self.fps_wait - elapsed);
        }

        self.fps_control = Instant::now();
        Ok(())
    }
}

unsafe impl Send for VideoReader {}
unsafe impl Sync for VideoReader {}
