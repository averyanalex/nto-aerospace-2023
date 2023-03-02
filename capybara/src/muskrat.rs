use tokio::sync::mpsc;

pub async fn run_muskrat(mut set_angle_rx: mpsc::Receiver<u8>) {
    while let Some(angle) = set_angle_rx.recv().await {
        println!("New angle = {}", angle);
    }
}
