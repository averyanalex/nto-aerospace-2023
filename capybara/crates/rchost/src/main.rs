use anyhow::Result;
use image::RgbImage;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinSet;

use camera::run_camera;
use common::init_log;
use common::wait_tasks;
use muskrat::run_muskrat;
use muskrat::servo::run_servo;
use proto::Odometry;
use rc::run_rc;
use ros::run_ros;
use ws::run_ws;

#[tokio::main]
async fn main() -> Result<()> {
    init_log();

    let (set_raw_angle_tx, set_raw_angle_rx) = mpsc::channel::<f64>(1);
    let (angle_tx, angle_rx) = watch::channel(2390.0);
    let (camera_tx, camera_rx) = watch::channel(RgbImage::new(640, 480));
    let (button_tx, _) = broadcast::channel(1);

    let (up_tx, _) = broadcast::channel(32);
    let (down_tx, down_rx) = broadcast::channel(32);

    let (odometry_tx, odometry_rx) = watch::channel(Odometry {
        x: 0.0,
        y: 0.0,
        theta: 0.0,
    });
    let (velocity_tx, velocity_rx) = broadcast::channel(1);

    let mut tasks = JoinSet::<Result<()>>::new();
    tasks.spawn(run_ros(odometry_tx, velocity_rx));
    tasks.spawn(run_muskrat(set_raw_angle_rx, button_tx));
    tasks.spawn(run_servo(angle_rx, set_raw_angle_tx));
    tasks.spawn(run_ws(up_tx.clone(), down_tx));
    tasks.spawn(run_camera(camera_tx));
    tasks.spawn(run_rc(
        down_rx,
        up_tx,
        angle_tx,
        velocity_tx,
        odometry_rx,
        camera_rx,
    ));

    wait_tasks(tasks).await;
    Ok(())
}
