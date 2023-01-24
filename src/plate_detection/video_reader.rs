use crossbeam::channel::unbounded;
use crossbeam::channel::Receiver;
use crossbeam::channel::Sender;
use opencv::core::Size;

use opencv::imgproc::resize;
use opencv::imgproc::INTER_LINEAR;
use opencv::prelude::Mat;
use opencv::prelude::MatTraitConst;

use opencv::prelude::VideoCaptureTrait;
use opencv::videoio::VideoCapture;

use opencv::videoio::CAP_ANY;
use rusted_pipe::channels::ChannelID;
use rusted_pipe::channels::WriteChannel;
use rusted_pipe::graph::Processor;
use rusted_pipe::packet::PacketSet;
use rusted_pipe::DataVersion;
use rusted_pipe::RustedPipeError;

use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;

pub struct VideoReader {
    id: String,
    capture: VideoCapture,
    fps_control: Instant,
    fps_wait: Duration,
    fps: u64,
    done_event_: Sender<bool>,
    done_event: Receiver<bool>,
}
impl VideoReader {
    pub fn default() -> Self {
        let capture =
            VideoCapture::from_file("data/210112_01_Covid Oxford_4k_061.mp4", CAP_ANY).unwrap();
        let fps = 2;
        let (done_event_, done_event) = unbounded();
        Self {
            id: "VideoReader".to_string(),
            capture,
            fps_control: Instant::now(),
            fps_wait: Duration::from_millis(1000 / fps),
            fps,
            done_event_,
            done_event,
        }
    }

    pub fn get_done_event(&self) -> Receiver<bool> {
        self.done_event.clone()
    }
}

impl Processor for VideoReader {
    fn handle(
        &mut self,
        mut _input: PacketSet,
        output_channel: Arc<Mutex<WriteChannel>>,
    ) -> Result<(), RustedPipeError> {
        let mut image = Mat::default();
        let grabbed = self.capture.read(&mut image).unwrap();

        if !grabbed || image.empty() {
            self.done_event_.send(true).unwrap();
            return Ok(());
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
        output_channel
            .lock()
            .unwrap()
            .write(
                &ChannelID::from("image"),
                image_resized,
                &DataVersion::from_now(),
            )
            .unwrap();
        let elapsed = self.fps_control.elapsed();

        if self.fps_wait > elapsed {
            thread::sleep(self.fps_wait - elapsed);
        }
        self.fps_control = Instant::now();
        Ok(())
    }

    fn id(&self) -> &String {
        return &self.id;
    }
}

unsafe impl Send for VideoReader {}
unsafe impl Sync for VideoReader {}
