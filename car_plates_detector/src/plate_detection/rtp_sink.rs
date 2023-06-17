use gstreamer::parse_launch;
use gstreamer::prelude::Cast;
use gstreamer::prelude::CastNone;
use gstreamer::prelude::ChildProxyExt;
use gstreamer::prelude::ElementExt;
use gstreamer::Buffer;
use gstreamer::Caps;
use gstreamer_video::VideoCapsBuilder;
use gstreamer_video::VideoFormat;
use opencv::prelude::Mat;
use opencv::prelude::MatTraitConstManual;
use rusted_pipe::{
    channels::{read_channel::InputGenerator, typed_read_channel::ReadChannel1},
    graph::processor::TerminalProcessor,
    RustedPipeError,
};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use gstreamer::prelude::MulDiv;
pub struct RtpSink {
    pub id: String,
    _pipeline: gstreamer::Pipeline,
    pub fps: usize,
    frames: usize,
    buffer: Buffer,
    buffer_s: Sender<Buffer>,
}

pub fn create_caps(width: usize, height: usize, fps: usize) -> Caps {
    VideoCapsBuilder::new()
        .width(width as i32)
        .height(height as i32)
        .framerate(gstreamer::Fraction::new(fps as i32, 1))
        .format(VideoFormat::Bgr)
        .build()
}

impl RtpSink {
    pub fn new(fps: usize, host: &str, port: usize) -> Self {
        gstreamer::init().unwrap();
        // let pipeline_str = format!(
        //     "appsrc ! videoconvert ! x264enc ! mpegtsmux ! filesink location=file.mp4"
        // );
        let pipeline_str =
            &format!("appsrc ! videoconvert ! x264enc tune=zerolatency bitrate=500 speed-preset=superfast ! rtph264pay ! udpsink host={host} port={port}");

        let pipeline = parse_launch(&pipeline_str)
            .expect(format!("Cannot create pipeline {pipeline_str}").as_str());

        let pipeline = pipeline.dynamic_cast::<gstreamer::Pipeline>().unwrap();

        let app_src = pipeline
            .child_by_index(4)
            .and_dynamic_cast::<gstreamer_app::AppSrc>()
            .expect("Cannot create AppSrc");

        app_src.set_caps(Some(&create_caps(1280, 720, fps)));
        app_src.set_format(gstreamer::Format::Time);
        let id = "rtp_sink".to_string();

        pipeline
            .set_state(gstreamer::State::Playing)
            .expect("Unable to set the pipeline to the `Playing` state");

        let buffer = Buffer::with_size(1280 * 720 * 3).expect("Cannot create gst buffer");
        let (buffer_s, buffer_r) = channel();
        let sink = Self {
            id,
            _pipeline: pipeline,
            fps,
            frames: 0,
            buffer,
            buffer_s,
        };
        sink.init(app_src, buffer_r);
        return sink;
    }

    fn init(&self, app_src: gstreamer_app::AppSrc, receiver: Receiver<Buffer>) {
        app_src.set_callbacks(
            // Since our appsrc element operates in pull mode (it asks us to provide data),
            // we add a handler for the need-data callback and provide new data from there.
            // In our case, we told gstreamer that we do 2 frames per second. While the
            // buffers of all elements of the pipeline are still empty, this will be called
            // a couple of times until all of them are filled. After this initial period,
            // this handler will be called (on average) twice per second.
            gstreamer_app::AppSrcCallbacks::builder()
                .need_data(move |appsrc, _| {
                    if let Ok(buffer) = receiver.recv() {
                        appsrc
                            .push_buffer(buffer)
                            .expect("Cannot push buffer to AppSrc");
                    }
                })
                .build(),
        );
    }
}

impl TerminalProcessor for RtpSink {
    type INPUT = ReadChannel1<Mat>;
    fn handle(
        &mut self,
        mut input: <Self::INPUT as InputGenerator>::INPUT,
    ) -> Result<(), RustedPipeError> {
        if let Some(image) = input.c1_owned() {
            let duration = gstreamer::format::ClockTime::SECOND
                .mul_div_floor(1 as u64, self.fps as u64)
                .expect("u64 overflow");
            let pts = duration * self.frames as u64;

            let image_data = image.data;
            let data = image_data.data_bytes().expect("Cannot read Mat bytes");

            self.buffer
                .make_mut()
                .copy_from_slice(0, data)
                .expect("Cannot copy to gst buffer");
            self.buffer.make_mut().set_duration(duration);
            self.buffer.make_mut().set_pts(pts);
            self.buffer.make_mut().set_dts(pts);
            println!("RTP Sinked frame {}", image.version.timestamp_ns);
            self.buffer_s
                .send(self.buffer.copy())
                .expect("Cannot write to AppSrc");
            self.frames += 1;
        }

        Ok(())
    }
}

unsafe impl Send for RtpSink {}
unsafe impl Sync for RtpSink {}
