use anyhow::Result;
use image::RgbImage;
use log::*;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::spawn_blocking;

pub async fn run_phototaker(
    mut photo_request_rx: mpsc::Receiver<()>,
    camera_rx: watch::Receiver<RgbImage>,
    photo_data_tx: broadcast::Sender<Vec<u8>>,
) -> Result<()> {
    while let Some(_) = photo_request_rx.recv().await {
        info!("taking photo");
        let img = (*camera_rx.borrow()).clone();

        let webp: Result<Vec<u8>> = spawn_blocking(move || {
            let mut webp_buf: Vec<u8> = Vec::new();
            img.write_to(
                &mut std::io::Cursor::new(&mut webp_buf),
                image::ImageOutputFormat::WebP,
            )?;
            Ok(webp_buf)
        })
        .await?;

        let _ = photo_data_tx.send(webp?);
    }
    Ok(())
}
