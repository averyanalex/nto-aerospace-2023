use anyhow::Result;
// use log::*;
use tokio::sync::{broadcast, mpsc};

pub async fn run_muskrat(
    mut set_angle_rx: mpsc::Receiver<f64>,
    _button_tx: broadcast::Sender<()>,
) -> Result<()> {
    while let Some(_angle) = set_angle_rx.recv().await {
        // debug!("set angle = {}", angle);
    }
    Ok(())
}
