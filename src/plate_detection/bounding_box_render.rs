use opencv::core::Point;
use opencv::core::Rect;
use opencv::core::Scalar;
use opencv::core::Size;
use opencv::core::ToInputOutputArray;
use opencv::core::Vector;
use opencv::imgproc::put_text;
use opencv::imgproc::FONT_HERSHEY_PLAIN;
use opencv::imgproc::LINE_8;
use opencv::imgproc::{rectangle, LineTypes};
use opencv::prelude::Mat;

use opencv::videoio::VideoWriter;
use opencv::videoio::VideoWriterTrait;

use rusted_pipe::channels::ChannelID;
use rusted_pipe::channels::WriteChannel;
use rusted_pipe::graph::Processor;
use rusted_pipe::packet::PacketSet;

use rusted_pipe::RustedPipeError;

use std::sync::Arc;
use std::sync::Mutex;

use crate::plate_detection::CarWithText;

pub struct BoundingBoxRender {
    id: String,
    writer: VideoWriter,
}
impl BoundingBoxRender {
    pub fn default() -> Self {
        Self {
            id: "BoundingBoxRender".to_string(),
            writer: VideoWriter::new(
                "output.avi",
                VideoWriter::fourcc('M', 'J', 'P', 'G').unwrap(),
                25.0,
                Size::new(640, 480),
                true,
            )
            .unwrap(),
        }
    }
}

impl Drop for BoundingBoxRender {
    fn drop(&mut self) {
        println!("Dropping BoundingBoxRender!");
        self.writer.release().unwrap();
    }
}

impl Processor for BoundingBoxRender {
    fn handle(
        &mut self,
        mut _input: PacketSet,
        _output_channel: Arc<Mutex<WriteChannel>>,
    ) -> Result<(), RustedPipeError> {
        let mut bboxes_packet = _input
            .get_channel_owned::<Vector<Rect>>(&ChannelID::from("cars"))
            .unwrap();
        let plates_packet = _input
            .get_channel_owned::<Vec<CarWithText>>(&ChannelID::from("plates"))
            .unwrap();
        let bboxes = bboxes_packet.data.as_mut();

        let mut image = _input
            .get_channel_owned::<Mat>(&ChannelID::from("image"))
            .unwrap();
        let color = Scalar::from((255.0, 0.0, 0.0));
        let color_red = Scalar::from((0.0, 255.0, 0.0));
        let thikness_px = 2;

        let mut im_array = image.data.input_output_array().unwrap();

        for bbox_i in 0..bboxes.len() {
            let bbox = bboxes.get(bbox_i).unwrap();
            for plate_i in 0..plates_packet.data.len() {
                let plate = plates_packet.data.get(plate_i).unwrap();
                let intersection = bbox & plate.car;
                let plate_text = plate.plate.as_ref().unwrap();
                if intersection.size() == plate.car.size() {
                    let header = Rect::new(bbox.x, bbox.y, bbox.width, 20);
                    rectangle(
                        &mut im_array,
                        header,
                        color,
                        -1,
                        LineTypes::LINE_4 as i32,
                        0,
                    )
                    .unwrap();
                    rectangle(
                        &mut im_array,
                        plate.car,
                        color_red,
                        thikness_px,
                        LineTypes::LINE_4 as i32,
                        0,
                    )
                    .unwrap();
                    put_text(
                        &mut im_array,
                        plate_text,
                        Point::new(bbox.x, bbox.y + 20),
                        FONT_HERSHEY_PLAIN,
                        2.0,
                        Scalar::from((255.0, 255.0, 255.0)),
                        2,
                        LINE_8,
                        false,
                    )
                    .unwrap();
                }
            }

            rectangle(
                &mut im_array,
                bbox,
                color,
                thikness_px,
                LineTypes::LINE_4 as i32,
                0,
            )
            .unwrap();
        }

        self.writer.write(&im_array).unwrap();
        Ok(())
    }

    fn id(&self) -> &String {
        return &self.id;
    }
}

unsafe impl Send for BoundingBoxRender {}
unsafe impl Sync for BoundingBoxRender {}
