use anyhow::Result;
use image::io::Reader;
use log::*;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::io::Cursor;
use tokio::sync::broadcast;
use tokio::task::spawn_blocking;

pub async fn run_photosaver(mut data_rx: broadcast::Receiver<Vec<u8>>) -> Result<()> {
    loop {
        let data = match data_rx.recv().await {
            Ok(d) => d,
            Err(broadcast::error::RecvError::Lagged(l)) => {
                error!("lagged for {l} packets");
                continue;
            }
            Err(_) => return Ok(()),
        };

        let res: Result<()> = spawn_blocking(move || {
            let mut reader = Reader::new(Cursor::new(data));
            reader.set_format(image::ImageFormat::WebP);
            let img = reader.decode()?;

            let img_name: String = thread_rng()
                .sample_iter(&Alphanumeric)
                .take(8)
                .map(char::from)
                .collect();
            img.save(format!("photos/{img_name}.jpg"))?;
            info!("saved photo {img_name}.jpg");
            Ok(())
        })
        .await?;
        res?;
    }
}
