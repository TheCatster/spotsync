FROM rust as builder
WORKDIR app
COPY . .

RUN cargo build --release

FROM rust as runtime
WORKDIR app
COPY --from=builder /app/target/release/spotsync /app
USER root

RUN apt update && apt install -y python3 python3-pip git ffmpeg
RUN git clone https://github.com/deepjyoti30/ytmdl
RUN cd ytmdl && git checkout unstable && pip3 install .
RUN mkdir -p /root/.cache
RUN mkdir -p /root/.config
RUN chown -R root /root/.cache /root/.config

ENTRYPOINT ["sh", "-c", "/app/spotsync"]
