
## Car Plate Reader

This is a more complex pipeline that reads realtime or offline car plates from a road video feed.

## Installation

Apart from the building with Cargo we need to install some OS support libraries.

Tesseract and Opencv

`apt update && apt install -y libopencv-dev clang libclang-dev libleptonica-dev libtesseract-dev tesseract-ocr-eng -y`

Gstreamer

`apt update && apt -y install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav libgstrtspserver-1.0-dev libges-1.0-dev libgstreamer-plugins-bad1.0-dev`

Finally run

`cargo build`