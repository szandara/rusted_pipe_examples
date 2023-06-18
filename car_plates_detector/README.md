
## Car Plate Reader

This is a complex pipeline that reads realtime or offline car plates from a road video feed.

Car detector and OCR can run in parallel as they work on the data independently but they produce at different speed. We can also run the sequential but the overall throughput would be slower.
Finally the result could look like this depending on the synchronization strategy (more on this below).

<img src="docs/offline.gif" width="500" height="320">


## Installation

Apart from the building with Cargo we need to install some OS support libraries.

Tesseract and Opencv

`apt update && apt install -y libopencv-dev clang libclang-dev libleptonica-dev libtesseract-dev tesseract-ocr-eng -y`

Gstreamer

`apt update && apt -y install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav libgstrtspserver-1.0-dev libges-1.0-dev libgstreamer-plugins-bad1.0-dev`

Note that if you want to run with GPU processing, you need to compile OpenCV with CUDA support.

Finally run

`cargo build`

## Run

Run (rtp output)

`cargo run --bin cars_realtime_wait`

or

Run (offline processing)

`cargo run --bin cars_offline`