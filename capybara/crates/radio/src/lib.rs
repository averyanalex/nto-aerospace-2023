use anyhow::Result;
use log::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time::{sleep, Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::broadcast,
};
use tokio_serial::{SerialPort, SerialPortBuilderExt};

use common::wait_tasks;

const GET_CONFIG_CMD: [u8; 3] = [0xAA, 0xFA, 0x01];
const SET_CONFIG_CMD: [u8; 18] = [
    0xAA, 0xFA, 0x03, // set config command
    7,    // RF Channel
    1,    // 433 MHz RF Band
    8,    // 115200 RF Rate
    7,    // +20 dBm RF Power
    7,    // 57600 Serial transmission rate
    2,    // 8 bits data bits
    1,    // 1 bits stop bits
    1,    // no parity
    0x22, 0xB4, 0xE6, 0x21, // NET ID
    0x00, 0x00, // NODE ID
    0x0A, // end of command
];

pub async fn run_radio(
    port_path: &str,
    mut send_rx: broadcast::Receiver<Vec<u8>>,
    receive_tx: broadcast::Sender<Vec<u8>>,
) -> Result<()> {
    if true {
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
    }

    let port = Arc::new(Mutex::new(
        tokio_serial::new(port_path, 57600).open_native_async()?,
    ));

    port.lock().await.set_exclusive(true)?;
    port.lock().await.write_data_terminal_ready(false)?;

    sleep(Duration::from_secs(3)).await;

    while port.lock().await.bytes_to_read()? > 0 {
        warn!("reading trash byte");
        port.lock().await.read_exact(&mut [0u8]).await?;
    }

    let rec_port = port.clone();

    let mut tasks = JoinSet::<Result<()>>::new();

    tasks.spawn(async move {
        loop {
            let data_to_send = match send_rx.recv().await {
                Ok(d) => d,
                Err(broadcast::error::RecvError::Lagged(l)) => {
                    error!("lagged for {l} packets");
                    continue;
                }
                Err(_) => return Ok(()),
            };
            debug!("sending {} bytes to radio", data_to_send.len());
            port.lock()
                .await
                .write_u32(data_to_send.len() as u32)
                .await?;
            port.lock().await.write_all(&data_to_send).await?;
        }
    });

    tasks.spawn(async move {
        loop {
            if rec_port.lock().await.bytes_to_read()? > 0 {
                let buf_len = rec_port.lock().await.read_u32().await?;
                let mut buf: Vec<u8> = vec![0; buf_len as usize];
                debug!("receiving {} bytes from radio", buf_len);
                rec_port.lock().await.read_exact(&mut buf).await?;
                let _ = receive_tx.send(buf);
            } else {
                sleep(Duration::from_millis(20)).await;
            }
        }
    });

    wait_tasks(tasks).await;

    Ok(())
}
