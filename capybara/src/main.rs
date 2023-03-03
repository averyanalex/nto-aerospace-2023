use futures::future::join_all;
use image::RgbImage;
use tokio::spawn;
use tokio::sync::{mpsc, watch};

mod camera;
mod muskrat;

#[tokio::main]
async fn main() {
    let (set_angle_tx, set_angle_rx) = mpsc::channel::<u8>(1);
    let musk = spawn(muskrat::run_muskrat(set_angle_rx));

    let (camera_tx, mut camera_rx) = watch::channel(RgbImage::new(32, 32));
    let camera = spawn(camera::run_camera(camera_tx));

    join_all(vec![musk, camera]).await;
}
