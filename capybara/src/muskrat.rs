use anyhow::Result;
// use log::*;
use tokio::sync::{broadcast, mpsc};
use tokio_serial::SerialPortBuilderExt;

pub async fn run_muskrat(
    mut _set_angle_rx: mpsc::Receiver<f64>,
    button_tx: broadcast::Sender<()>,
) -> Result<()> {
    let mut port = tokio_serial::new(
        "/dev/serial/by-path/platform-fd500000.pcie-pci-0000:01:00.0-usb-0:1.4.1:1.0-port0",
        115200,
    )
    .open_native_async()?;
    port.set_exclusive(true)?;

    loop {
        port.readable().await?;
        let mut buf = [0u8; 1];
        port.try_read(&mut buf)?;
        if buf[0] == 0x07 {
            let _ = button_tx.send(());
        }
    }
    // while let Some(_angle) = set_angle_rx.recv().await {
    //     // debug!("set angle = {}", angle);
    // }
}
