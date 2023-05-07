use std::time::Duration;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use opencv::core::Point;
use opencv::core::Rect;
use opencv::core::Scalar;
use opencv::core::Size;
use opencv::core::Vector;
use opencv::imgproc::put_text;
use opencv::imgproc::FONT_HERSHEY_PLAIN;
use opencv::imgproc::LINE_8;
use opencv::imgproc::{rectangle, LineTypes};
use opencv::prelude::Mat;

use lazy_static::lazy_static;
use opencv::videoio::VideoWriter;
use opencv::videoio::VideoWriterTrait;
use prometheus::register_histogram;
use prometheus::Histogram;
use rusted_pipe::channels::read_channel::InputGenerator;
use rusted_pipe::channels::typed_read_channel::ReadChannel3;
use rusted_pipe::channels::typed_write_channel::WriteChannel1;
use rusted_pipe::graph::processor::Processor;
use rusted_pipe::graph::processor::ProcessorWriter;
use rusted_pipe::RustedPipeError;

use crate::plate_detection::CarWithText;

pub struct BoundingBoxRender {
    writer: Option<VideoWriter>,
    metrics: &'static Histogram,
}
lazy_static! {
    static ref METRICS: Histogram =
        register_histogram!("full_cycle", format!("Timing for the full pipeline"))
            .expect(&format!("Cannot create pipeline timer"));
}
impl BoundingBoxRender {
    pub fn with_save_to_file() -> Self {
        Self {
            writer: Some(
                VideoWriter::new(
                    "output.avi",
                    VideoWriter::fourcc('M', 'J', 'P', 'G').unwrap(),
                    25.0,
                    Size::new(1280, 720),
                    true,
                )
                .unwrap(),
            ),
            metrics: &METRICS,
        }
    }

    pub fn default() -> Self {
        Self {
            writer: None,
            metrics: &METRICS,
        }
    }
}

impl Drop for BoundingBoxRender {
    fn drop(&mut self) {
        println!("Dropping BoundingBoxRender!");
        if let Some(writer) = self.writer.as_mut() {
            writer.release().unwrap();
        }
    }
}

impl Processor for BoundingBoxRender {
    type INPUT = ReadChannel3<Vector<Rect>, Vec<CarWithText>, Mat>;
    type OUTPUT = WriteChannel1<Mat>;
    fn handle(
        &mut self,
        mut input: <Self::INPUT as InputGenerator>::INPUT,
        mut output: ProcessorWriter<Self::OUTPUT>,
    ) -> Result<(), RustedPipeError> {
        if let Some(image) = input.c3() {
            println!("Render Image {}", image.version.timestamp_ns);
        } else {
            println!("Skipping inferred data with no image");
            return Ok(());
        }

        let mut image = input.c3_owned().unwrap();

        let mut plates = Vec::<CarWithText>::new();
        let mut bboxes = Vector::<Rect>::new();
        if let Some(bboxes_packet) = input.c1_owned() {
            bboxes = bboxes_packet.data;
        }
        if let Some(plates_packet) = input.c2_owned() {
            plates = plates_packet.data;
        }

        let color = Scalar::from((255.0, 0.0, 0.0));
        let color_red = Scalar::from((0.0, 255.0, 0.0));
        let thikness_px = 2;

        for bbox_i in 0..bboxes.len() {
            let bbox = bboxes.get(bbox_i).unwrap();

            rectangle(
                &mut image.data,
                bbox,
                color,
                thikness_px,
                LineTypes::LINE_4 as i32,
                0,
            )
            .unwrap();
        }

        for plate_i in 0..plates.len() {
            let plate = plates.get(plate_i).unwrap();
            let plate_text = plate.plate.as_ref().unwrap();
            let header = Rect::new(plate.car.x, plate.car.y - 60, plate.car.width, 60);
            rectangle(
                &mut image.data,
                header,
                color,
                -1,
                LineTypes::LINE_4 as i32,
                0,
            )
            .unwrap();
            rectangle(
                &mut image.data,
                plate.car,
                color_red,
                thikness_px,
                LineTypes::LINE_4 as i32,
                0,
            )
            .unwrap();
            put_text(
                &mut image.data,
                plate_text,
                Point::new(plate.car.x, plate.car.y - 3),
                FONT_HERSHEY_PLAIN,
                4.0,
                Scalar::from((255.0, 255.0, 255.0)),
                2,
                LINE_8,
                false,
            )
            .unwrap();
        }

        if let Some(writer) = self.writer.as_mut() {
            writer.write(&image.data).unwrap();
        }

        output
            .writer
            .c1()
            .write(image.data, &image.version)
            .expect("Cannot write to output buffer");
        let pipeline_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
            - Duration::from_nanos(image.version.timestamp_ns as u64);

        println!("Pipe time {}", pipeline_time.as_secs_f64());
        self.metrics.observe(pipeline_time.as_secs_f64());
        Ok(())
    }
}

unsafe impl Send for BoundingBoxRender {}
unsafe impl Sync for BoundingBoxRender {}
