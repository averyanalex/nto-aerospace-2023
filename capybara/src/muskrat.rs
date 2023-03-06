use anyhow::Result;
use tokio::sync::mpsc;

pub async fn run_muskrat(mut set_angle_rx: mpsc::Receiver<f64>) -> Result<()> {
    while let Some(angle) = set_angle_rx.recv().await {
        println!("New angle = {}", angle);
    }
    Ok(())
}
