ARG TARGET=arm-unknown-linux-gnueabihf
FROM ghcr.io/cross-rs/$TARGET:main as BUILD

ARG TARGET
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"
RUN rustup target add $TARGET

WORKDIR /build
ADD . .
RUN cargo build --release --target $TARGET
