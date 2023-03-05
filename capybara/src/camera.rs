use image::io::Reader as ImageReader;
use image::RgbImage;
use std::io::Cursor;
use tokio::sync::broadcast;
use tokio::task::spawn_blocking;

pub async fn run_camera(camera_tx: broadcast::Sender<RgbImage>) {
    let mut camera = rscam::new("/dev/video0").unwrap();

    camera
        .start(&rscam::Config {
            interval: (1, 30),
            resolution: (640, 480),
            format: b"MJPG",
            ..Default::default()
        })
        .unwrap();

    spawn_blocking(move || loop {
        // camera.capture().unwrap();
        // camera.capture().unwrap();
        let frame = camera.capture().unwrap();
        let decoded_frame = ImageReader::new(Cursor::new(&frame[..]))
            .with_guessed_format()
            .unwrap()
            .decode()
            .unwrap()
            .into_rgb8();
        camera_tx.send(decoded_frame).unwrap();
    })
    .await
    .unwrap();
}
