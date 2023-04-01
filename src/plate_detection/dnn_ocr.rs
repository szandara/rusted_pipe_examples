use super::CarWithText;

use opencv::core::Point;
use opencv::core::Point2f;

use opencv::core::Rect;
use opencv::core::Scalar;
use opencv::core::Size;

use opencv::core::Vector;
use opencv::core::BORDER_CONSTANT;
use opencv::core::CV_32F;

use opencv::dnn::read_net;

use opencv::dnn::TextDetectionModel_EAST;
use opencv::dnn::TextDetectionModel_EASTTrait;
use opencv::dnn::TextRecognitionModel;

use opencv::imgproc::cvt_color;
use opencv::imgproc::get_perspective_transform;

use opencv::imgproc::warp_perspective;
use opencv::imgproc::COLOR_BGR2GRAY;

use opencv::imgproc::INTER_LINEAR;
use opencv::prelude::Mat;

use opencv::prelude::MatTraitConst;

use opencv::prelude::ModelTrait;

use opencv::prelude::TextDetectionModelTraitConst;
use opencv::prelude::TextRecognitionModelTrait;
use opencv::prelude::TextRecognitionModelTraitConst;

use rusted_pipe::channels::typed_read_channel::ReadChannel1;
use rusted_pipe::channels::typed_write_channel::TypedWriteChannel;
use rusted_pipe::channels::typed_write_channel::WriteChannel1;
use rusted_pipe::graph::processor::Processor;
use rusted_pipe::packet::typed::ReadChannel1PacketSet;
use rusted_pipe::RustedPipeError;

use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::sync::MutexGuard;

pub struct DnnOcrReader {
    id: String,
    network: TextDetectionModel_EAST,
    ocr: TextRecognitionModel,
}
impl DnnOcrReader {
    pub fn default() -> Self {
        let net = read_net("models/frozen_east_text_detection.pb", "", "").unwrap();
        let mut net = TextDetectionModel_EAST::new(&net).unwrap();

        // Set the input of the network
        net.set_confidence_threshold(0.5)
            .unwrap()
            .set_nms_threshold(0.4)
            .unwrap();

        let scale = 1.0;
        let mean = Scalar::from((123.68, 116.78, 103.94));
        let input_size = Size::new(320, 320);

        net.set_input_params(scale, input_size, mean, true, false)
            .unwrap();

        let scale = 1.0 / 127.5;
        let mean = Scalar::from((127.5, 127.5, 127.5));
        let input_size = Size::new(100, 32);

        let mut ocr =
            TextRecognitionModel::from_file("models/CRNN_VGG_BiLSTM_CTC.onnx", "").unwrap();
        ocr.set_decode_type("CTC-greedy").unwrap();

        let mut vocabulary = Vector::<String>::default();
        let file = File::open("models/alphabet_36.txt").unwrap();
        let reader = BufReader::new(file);

        for line in reader.lines() {
            vocabulary.push(&line.unwrap());
        }
        ocr.set_vocabulary(&vocabulary).unwrap();
        ocr.set_input_params(scale, input_size, mean, false, false)
            .unwrap();

        Self {
            id: "DnnOcrReader".to_string(),
            network: net,
            ocr,
        }
    }

    fn reshape_plate(&self, image: &Mat, rect: &Vector<Point>) -> Mat {
        let output_size = Size::new(100, 32);
        let mut image_2f = Mat::default();
        image.convert_to(&mut image_2f, CV_32F, 1.0, 0.0).unwrap();
        let mut rect_2f = Vector::<Point2f>::default();
        for p in rect {
            rect_2f.push(Point2f::new(p.x as f32, p.y as f32));
        }

        let mut target_rect_2f = Vector::<Point2f>::default();
        target_rect_2f.push(Point2f::new(0.0, output_size.height as f32 - 1.0));
        target_rect_2f.push(Point2f::new(0.0, 0.0));
        target_rect_2f.push(Point2f::new(output_size.width as f32 - 1.0, 0.0));
        target_rect_2f.push(Point2f::new(
            output_size.width as f32 - 1.0,
            output_size.height as f32 - 1.0,
        ));

        let perspective = get_perspective_transform(&rect_2f, &target_rect_2f, 0).unwrap();
        let mut output = Mat::default();

        warp_perspective(
            &image_2f,
            &mut output,
            &perspective,
            output_size,
            INTER_LINEAR,
            BORDER_CONSTANT,
            Scalar::default(),
        )
        .unwrap();
        return output;
    }
}

unsafe impl Send for DnnOcrReader {}
unsafe impl Sync for DnnOcrReader {}

impl Processor<ReadChannel1<Mat>> for DnnOcrReader {
    type WRITE = WriteChannel1<Vec<CarWithText>>;
    fn handle(
        &mut self,
        input: ReadChannel1PacketSet<Mat>,
        mut output_channel: MutexGuard<TypedWriteChannel<Self::WRITE>>,
    ) -> Result<(), RustedPipeError> {
        let image_packet = input.c1().unwrap();
        let image = &image_packet.data;
        let mut grey = Mat::default();
        cvt_color(image, &mut grey, COLOR_BGR2GRAY, 0).unwrap();

        let mut output = Vector::<Vector<Point>>::default();
        self.network.detect(image, &mut output).unwrap();

        let mut out_rect: Vec<CarWithText> = vec![];
        println!("Processing OCR frame");
        for r in output {
            let rect = Rect::new(
                r.get(1).unwrap().x,
                r.get(1).unwrap().y,
                r.get(3).unwrap().x - r.get(1).unwrap().x,
                r.get(3).unwrap().y - r.get(1).unwrap().y,
            );
            if rect.x > 20
                && rect.y > 20
                && rect.x <= image.cols() - 20
                && rect.y <= image.rows() - 20
            {
                let cropped = self.reshape_plate(&grey, &r);
                let result = self.ocr.recognize(&cropped).unwrap();
                //println!("{:?}, {:?}, {:?}", result, r, image.mat_size());
                out_rect.push(CarWithText::new(Some(result), rect));
            }
        }

        output_channel
            .writer
            .c1()
            .write(out_rect, &image_packet.version)
            .unwrap();

        Ok(())
    }

    fn id(&self) -> &String {
        return &self.id;
    }
}
