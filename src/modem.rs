use bytes::{BufMut, BytesMut};
use futures::SinkExt;
use std::{
    io::{self, Error},
    str,
};
use tokio::time::{sleep, Duration};
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::{Decoder, Encoder};

struct LineCodec;

impl Decoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let newline = src.as_ref().iter().position(|b| *b == b'\n');
        if let Some(n) = newline {
            let line = src.split_to(n + 1);
            return match str::from_utf8(line.as_ref()) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Invalid String")),
            };
        }
        Ok(None)
    }
}

impl Encoder<String> for LineCodec {
    type Error = io::Error;

    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        log::info!("Sending: {:?}", &item);
        dst.reserve(item.len() + 2);
        dst.put(item.as_bytes());
        dst.put_u8(b'\r');
        dst.put_u8(b'\n');
        Ok(())
    }
}

pub struct Modem {
    stream: tokio_util::codec::Framed<tokio_serial::SerialStream, LineCodec>,
}

impl Modem {
    pub fn new(serial_port: &str, baud_rate: u32) -> Self {
        let port = tokio_serial::new(serial_port, baud_rate)
            .open_native_async()
            .expect("Failed to open serial port.");

        Modem {
            stream: LineCodec.framed(port),
        }
    }

    pub async fn send_string(&mut self, data: String) -> Result<(), Error> {
        self.stream.send(data).await
    }

    pub async fn send_open_door_cmd(&mut self) -> Result<(), Error> {
        self.stream.send(String::from("ATX1")).await?;
        sleep(Duration::from_millis(100)).await;
        self.stream.send(String::from("ATD*1240")).await?;
        sleep(Duration::from_secs(10)).await;
        self.stream.send(String::from("ATD")).await
    }
}
