FROM arm64v8/rust:1.66.1

RUN apt update && apt install -y libopencv-dev clang libclang-dev libleptonica-dev libtesseract-dev tesseract-ocr-eng

RUN apt update && apt -y install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev \
      gstreamer1.0-plugins-base gstreamer1.0-plugins-good \
      gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly \
      gstreamer1.0-libav libgstrtspserver-1.0-dev libges-1.0-dev libgstreamer-plugins-bad1.0-dev

RUN rustup default nightly
RUN rustup component add rustfmt
ENV CARGO_BUILD_TARGET_DIR=/root/target