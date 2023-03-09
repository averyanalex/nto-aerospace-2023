use anyhow::Result;
// use log::*;
use log::*;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::task::JoinSet;
use tokio::time::{sleep, Duration};
use tokio_serial::SerialPort;
use tokio_serial::SerialPortBuilderExt;

use common::wait_tasks;

pub mod servo;

pub async fn run_muskrat(
    mut set_angle_rx: mpsc::Receiver<f64>,
    button_tx: broadcast::Sender<()>,
) -> Result<()> {
    let mut port = tokio_serial::new(
        "/dev/serial/by-path/platform-fd500000.pcie-pci-0000:01:00.0-usb-0:1.4.1:1.0-port0",
        115200,
    )
    .open_native_async()?;
    port.set_exclusive(true)?;

    let port = Arc::new(Mutex::new(port));

    let rec_port = port.clone();

    let mut tasks = JoinSet::<Result<()>>::new();

    tasks.spawn(async move {
        loop {
            let angle = match set_angle_rx.recv().await {
                Some(d) => d as u32,
                None => return Ok(()),
            };
            let mut p = port.lock().await;
            debug!("sending {} angle", angle);
            p.write_all(&[0x03]).await?;
            p.write_u32(angle).await?;
        }
    });

    tasks.spawn(async move {
        loop {
            let mut p = rec_port.lock().await;
            if p.bytes_to_read()? > 0 {
                let mut buf = [0u8; 1];
                p.read_exact(&mut buf).await?;
                if buf[0] == 0x07 {
                    let _ = button_tx.send(());
                }
            }
            sleep(Duration::from_millis(20)).await;
        }
    });
    // while let Some(_angle) = set_angle_rx.recv().await {
    //     // debug!("set angle = {}", angle);
    // }
    wait_tasks(tasks).await;

    Ok(())
}
