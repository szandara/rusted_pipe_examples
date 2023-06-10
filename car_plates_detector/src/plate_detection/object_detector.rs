use opencv::core::min_max_loc;
use opencv::core::Point;
use opencv::core::Range;
use opencv::core::Rect;
use opencv::core::Scalar;
use opencv::core::Size;
use opencv::core::ToInputOutputArray;
use opencv::core::Vector;

use opencv::core::CV_32F;
use opencv::dnn;
use opencv::dnn::blob_from_image;
use opencv::dnn::nms_boxes;

use opencv::dnn::read_net_from_darknet;
use opencv::dnn::Net;
use opencv::dnn::DNN_BACKEND_CUDA;
use opencv::dnn::DNN_BACKEND_OPENCV;

use opencv::dnn::DNN_TARGET_CPU;
use opencv::dnn::DNN_TARGET_CUDA;

use opencv::prelude::Mat;

use opencv::prelude::MatTraitConst;

use opencv::prelude::NetTrait;
use opencv::prelude::NetTraitConst;
use rusted_pipe::channels::read_channel::InputGenerator;
use rusted_pipe::channels::typed_read_channel::ReadChannel1;
use rusted_pipe::channels::typed_write_channel::WriteChannel1;
use rusted_pipe::graph::processor::Processor;
use rusted_pipe::graph::processor::ProcessorWriter;
use rusted_pipe::RustedPipeError;

pub struct ObjectDetector {
    classifier: Net,
    input_size: i32,
    post_processor: &'static dyn YoloProcessor,
}

trait YoloProcessor {
    fn post_process(&self, img_rows: i32, img_cols: i32, outputs: &Vector<Mat>) -> Vector<Rect>;
}

struct YoloProcessorV3 {}

impl YoloProcessor for YoloProcessorV3 {
    fn post_process(&self, img_rows: i32, img_cols: i32, outputs: &Vector<Mat>) -> Vector<Rect> {
        let mut confidences = Vector::<f32>::default();
        let mut boxes = Vector::<Rect>::default();

        println!("{:?}", outputs);
        for data in outputs {
            for j in 0..data.rows() {
                let mut scores = data
                    .row(j)
                    .unwrap()
                    .col_range(&Range::new(5, data.cols()).unwrap())
                    .unwrap();

                let mut min_val_p = 0.0;
                let mut max_loc_p = Point::default();
                let min_val = None;
                let max_val = Some(&mut min_val_p);
                let min_loc = None;
                let max_loc = Some(&mut max_loc_p);
                let scores_ia = &scores.input_output_array().unwrap();
                min_max_loc(
                    scores_ia,
                    min_val,
                    max_val,
                    min_loc,
                    max_loc,
                    &Mat::default().input_output_array().unwrap(),
                )
                .unwrap();
                let max_conf = min_val_p as f32;
                if max_conf > 0.5 && max_loc_p.x == 2 {
                    let center_x: i32 =
                        (data.at_2d::<f32>(j, 0).unwrap() * img_cols as f32).round() as i32;
                    let center_y: i32 =
                        (data.at_2d::<f32>(j, 1).unwrap() * img_rows as f32).round() as i32;
                    let width: i32 =
                        (data.at_2d::<f32>(j, 2).unwrap() * img_cols as f32).round() as i32;
                    let height: i32 =
                        (data.at_2d::<f32>(j, 3).unwrap() * img_rows as f32).round() as i32;
                    let left = center_x - width / 2;
                    let top = center_y - height / 2;

                    confidences.push(max_conf);
                    boxes.push(Rect::new(left, top, width, height));
                }
            }
        }

        let mut indices = Vector::<i32>::default();
        nms_boxes(&boxes, &confidences, 0.5, 0.4, &mut indices, 1.0, 0).unwrap();

        let mut output = Vector::<Rect>::default();
        for i in indices {
            output.push(boxes.get(i as usize).unwrap());
        }

        return output;
    }
}

struct YoloProcessorV5 {
    input_size: i32,
}

impl YoloProcessor for YoloProcessorV5 {
    fn post_process(&self, img_rows: i32, img_cols: i32, outputs: &Vector<Mat>) -> Vector<Rect> {
        let mut confidences = Vector::<f32>::default();
        let mut boxes = Vector::<Rect>::default();

        let x_factor = img_cols as f32 / self.input_size as f32;
        let y_factor = img_rows as f32 / self.input_size as f32;

        for data in outputs {
            for j in 0..25200 {
                let confidence: f32 = *data.at_3d(0, j, 4).unwrap();
                if confidence > 0.4 {
                    let class_score: f32 = *data.at_3d(0, j, 5).unwrap();
                    if class_score > 0.25 {
                        let cx: f32 = *data.at_3d(0, j, 0).unwrap();
                        let cy: f32 = *data.at_3d(0, j, 1).unwrap();
                        let w: f32 = *data.at_3d(0, j, 2).unwrap();
                        let h: f32 = *data.at_3d(0, j, 3).unwrap();
                        let left = (cx - 0.5 * w) * x_factor;
                        let top = (cy - 0.5 * h) * y_factor;
                        let width = w * x_factor;
                        let height = h * y_factor;

                        confidences.push(confidence);
                        boxes.push(Rect::new(
                            left as i32,
                            top as i32,
                            width as i32,
                            height as i32,
                        ));
                    }
                }
            }
        }

        let mut indices = Vector::<i32>::default();
        nms_boxes(&boxes, &confidences, 0.5, 0.4, &mut indices, 1.0, 0).unwrap();

        let mut output = Vector::<Rect>::default();
        for i in indices {
            output.push(boxes.get(i as usize).unwrap());
        }

        return output;
    }
}

impl ObjectDetector {
    pub fn car_detector(use_gpu: bool) -> Self {
        let mut classifier =
            read_net_from_darknet("models/yolov3.cfg", "models/yolov3.weights").unwrap();

        if use_gpu {
            classifier.set_preferable_backend(DNN_BACKEND_CUDA).unwrap();
            classifier.set_preferable_target(DNN_TARGET_CUDA).unwrap();
        } else {
            classifier.set_preferable_backend(DNN_BACKEND_OPENCV).unwrap();
            classifier.set_preferable_target(DNN_TARGET_CPU).unwrap();
        }
        
        return ObjectDetector {
            classifier,
            input_size: 416,
            post_processor: &YoloProcessorV3 {},
        };
    }

    pub fn plate_detector(use_gpu: bool) -> Self {
        let mut classifier = dnn::read_net_from_onnx("models/plate_best.onnx").unwrap();

        if use_gpu {
            classifier.set_preferable_backend(DNN_BACKEND_CUDA).unwrap();
            classifier.set_preferable_target(DNN_TARGET_CUDA).unwrap();
        } else {
            classifier.set_preferable_backend(DNN_BACKEND_OPENCV).unwrap();
            classifier.set_preferable_target(DNN_TARGET_CPU).unwrap();
        }

        return ObjectDetector {
            classifier,
            input_size: 640,
            post_processor: &YoloProcessorV5 { input_size: 640 },
        };
    }
}

unsafe impl Send for ObjectDetector {}
unsafe impl Sync for ObjectDetector {}

impl Processor for ObjectDetector {
    type OUTPUT = WriteChannel1<Vector<Rect>>;
    type INPUT = ReadChannel1<Mat>;
    fn handle(
        &mut self,
        input: <Self::INPUT as InputGenerator>::INPUT,
        mut output: ProcessorWriter<Self::OUTPUT>,
    ) -> Result<(), RustedPipeError> {
        let image_packet = &input.c1().unwrap();
        println!("Object Detection {}", image_packet.version.timestamp_ns);

        let image = &image_packet.data;

        let mut blob = blob_from_image(
            &image,
            1.0 / 255.0,
            Size::new(self.input_size, self.input_size),
            Scalar::default(),
            true,
            false,
            CV_32F,
        )
        .unwrap();

        // Set the input of the network
        self.classifier
            .set_input(&mut blob, "", 1.0, Scalar::default())
            .unwrap();

        let output_names = self.classifier.get_unconnected_out_layers_names().unwrap();

        let mut output_values = Vector::<Mat>::default();

        self.classifier
            .forward(&mut output_values, &output_names)
            .unwrap();
        let out = self
            .post_processor
            .post_process(image.rows(), image.cols(), &output_values);

        //let out = Vector::<Rect>::default();
        output
            .writer
            .c1()
            .write(out, &image_packet.version)
            .unwrap();
        Ok(())
    }
}
