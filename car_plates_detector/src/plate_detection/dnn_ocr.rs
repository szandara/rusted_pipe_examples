use super::CarWithText;

use leptess::tesseract;
use leptess::tesseract::TessApi;
use opencv::core::Point;
use opencv::core::Rect;

use opencv::core::Vector;

use opencv::core::CV_32F;
use opencv::imgproc::cvt_color;

use opencv::imgproc::COLOR_BGR2GRAY;

use opencv::imgproc::filter_2d;
use opencv::prelude::Mat;

use opencv::prelude::MatTraitConst;

use opencv::prelude::MatTraitManual;
use rusted_pipe::channels::typed_read_channel::ReadChannel2;
use rusted_pipe::channels::typed_write_channel::WriteChannel1;
use rusted_pipe::graph::processor::Processor;
use rusted_pipe::graph::processor::ProcessorWriter;
use rusted_pipe::packet::typed::ReadChannel2PacketSet;
use rusted_pipe::RustedPipeError;
use std::ffi::CString;

pub struct DnnOcrReader {
    ocr: TessApi,
    deblur: bool,
}

impl DnnOcrReader {
    pub fn default() -> Self {
        let mut api = tesseract::TessApi::new(Some("models"), "licence").unwrap();
        let data_path_cstr = CString::new("models").unwrap();
        let lang = CString::new("licence").unwrap();

        api.raw
            .init_4(Some(data_path_cstr.as_ref()), Some(lang.as_ref()), 1)
            .unwrap();
        api.raw
            .set_variable(
                &CString::new("tessedit_char_whitelist").unwrap(),
                &CString::new("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789").unwrap(),
            )
            .unwrap();
        api.raw
            .set_variable(
                &CString::new("tessedit_pageseg_mode").unwrap(),
                &CString::new("7").unwrap(),
            )
            .unwrap();

        Self {
            ocr: api,
            deblur: false,
        }
    }

    fn reshape_plate(&self, image: &Mat, rect: &Rect) -> Mat {
        let mut image_2f = Mat::default();
        image.convert_to(&mut image_2f, CV_32F, 1.0, 0.0).unwrap();
        let mut rect_smaller = rect.clone();

        rect_smaller.x += (rect_smaller.width as f32 * 0.10) as i32;
        rect_smaller.y += (rect_smaller.height as f32 * 0.12) as i32;

        rect_smaller.width -= (rect_smaller.width as f32 * 0.12) as i32;
        rect_smaller.height -= (rect_smaller.height as f32 * 0.24) as i32;

        let cropped = image.apply_1(rect_smaller).unwrap();
        if self.deblur {
            let mut processed = Mat::default();
            let kernel = Mat::from_slice_2d(&[[-1, -1, -1], [-1, 9, -1], [-1, -1, -1]]).unwrap();

            filter_2d(
                &cropped,
                &mut processed,
                -1,
                &kernel,
                Point::new(0, 0),
                0.0,
                -22,
            )
            .unwrap();

            return processed;
        } else {
            // Make it contiguous
            return cropped.clone();
        }
    }
}

unsafe impl Send for DnnOcrReader {}
unsafe impl Sync for DnnOcrReader {}

impl Processor for DnnOcrReader {
    type INPUT = ReadChannel2<Mat, Vector<Rect>>;
    type OUTPUT = WriteChannel1<Vec<CarWithText>>;
    fn handle(
        &mut self,
        mut input: ReadChannel2PacketSet<Mat, Vector<Rect>>,
        mut output: ProcessorWriter<Self::OUTPUT>,
    ) -> Result<(), RustedPipeError> {
        let image_packet = input.c1_owned().unwrap();
        println!("OCR Image {}", image_packet.version.timestamp_ns);
        let image = &image_packet.data;
        let mut grey = Mat::default();
        cvt_color(image, &mut grey, COLOR_BGR2GRAY, 0).unwrap();

        let mut out_rect: Vec<CarWithText> = vec![];
        let plates = input.c2_owned().unwrap();
        for rect in plates.data {
            let ratio = rect.width as f32 / rect.height as f32;
            if rect.x > 2
                && rect.y > 2
                && rect.x <= image.cols() - 2
                && rect.y <= image.rows() - 2
                && ratio > 3.0
                && ratio < 4.0
            {
                let mut cropped = self.reshape_plate(&grey, &rect);

                let cols = cropped.cols();
                let rows = cropped.rows();
                self.ocr
                    .raw
                    .set_image(&cropped.data_bytes_mut().unwrap(), cols, rows, 1, cols)
                    .unwrap();
                let result = self.ocr.get_utf8_text().unwrap();

                println!("OCR {:?}, {:?}", result.trim(), cropped);
                out_rect.push(CarWithText::new(Some(String::from(result.trim())), rect));
            }
        }

        output
            .writer
            .c1()
            .write(out_rect, &image_packet.version)
            .unwrap();

        Ok(())
    }
}
