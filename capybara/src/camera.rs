use anyhow::Result;
use image::io::Reader as ImageReader;
use image::RgbImage;
use std::io::Cursor;
use tokio::sync::watch;
use tokio::task::spawn_blocking;

pub async fn run_camera(camera_tx: watch::Sender<RgbImage>) -> Result<()> {
    let mut camera = rscam::new("/dev/video0")?;

    camera.start(&rscam::Config {
        interval: (1, 30),
        resolution: (640, 480),
        format: b"MJPG",
        ..Default::default()
    })?;

    spawn_blocking(move || loop {
        camera.capture()?;
        camera.capture()?;
        let frame = camera.capture()?;
        let decoded_frame = ImageReader::new(Cursor::new(&frame[..]))
            .with_guessed_format()?
            .decode()?
            .into_rgb8();
        let _ = camera_tx.send(decoded_frame);
    })
    .await?
}
