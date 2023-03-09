use anyhow::Result;
use log::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{sleep, Duration};
use tokio_serial::{SerialPort, SerialPortBuilderExt};

const GET_CONFIG_CMD: [u8; 3] = [0xAA, 0xFA, 0x01];
const SET_CONFIG_CMD: [u8; 18] = [
    0xAA, 0xFA, 0x03, // set config command
    7,    // RF Channel
    1,    // 433 MHz RF Band
    7,    // 57600 RF Rate
    7,    // +20 dBm RF Power
    7,    // 57600 Serial transmission rate
    2,    // 8 bits data bits
    1,    // 1 bits stop bits
    1,    // no parity
    0x00, 0x00, 0x00, 0x00, // NET ID
    0x00, 0x00, // NODE ID
    0x0A, // end of command
];

#[tokio::main]
pub async fn main() -> Result<()> {
    let port_path = &std::env::args().collect::<Vec<_>>()[1];
    let mut port = tokio_serial::new(port_path, 9600).open_native_async()?;
    port.set_exclusive(true)?;
    port.write_data_terminal_ready(true)?;

    sleep(Duration::from_secs(3)).await;

    while port.bytes_to_read()? > 0 {
        warn!("reading trash byte");
        port.read_exact(&mut [0u8]).await?;
    }

    port.write_all(&GET_CONFIG_CMD).await?;
    let mut settings = [0u8; 16];
    port.read_exact(&mut settings).await?;
    info!("radio config: {:X?}", settings);

    port.write_all(&SET_CONFIG_CMD).await?;
    let mut res_buf = [0u8; 4];
    port.read_exact(&mut res_buf).await?;
    if &res_buf == b"OK\r\n" {
        info!("set radio config");
    } else {
        error!("error setting radio config");
    }
    while port.bytes_to_read()? > 0 {
        warn!("reading trash byte");
        port.read_exact(&mut [0u8]).await?;
    }

    port.write_all(&GET_CONFIG_CMD).await?;
    let mut settings = [0u8; 16];
    port.read_exact(&mut settings).await?;
    info!("radio config: {:X?}", settings);

    Ok(())
}
