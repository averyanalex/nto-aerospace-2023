use tokio::sync::broadcast;

pub async fn run_radio(mut encoder_rx: broadcast::Receiver<Vec<u8>>) {
    loop {
        let video_packet = encoder_rx.recv().await.unwrap();
        println!("Video len = {}", video_packet.len());
    }
}
