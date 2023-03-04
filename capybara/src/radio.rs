use tokio::sync::broadcast;
use tokio::time::Instant;
use log::*;

pub async fn run_radio(mut encoder_rx: broadcast::Receiver<Vec<u8>>) {
    let start = Instant::now();
    let mut total_bytes = 0u32;
    loop {
        let video_packet = encoder_rx.recv().await.unwrap();
        total_bytes += video_packet.len() as u32;
        let bitrate = 8.0 * (total_bytes as f32 / (Instant::now() - start).as_secs_f32());
        info!("Bitrate = {}", bitrate);
    }
}
