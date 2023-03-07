use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use image::RgbImage;
use log::*;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinSet;

use capybara::camera;
use capybara::encoder;
use capybara::muskrat;
use capybara::phototaker;
use capybara::radio;
use capybara::ros;
use capybara::servo;
use capybara::wait_tasks;
use capybara::ws;
use capybara::{Odometry, Velocity};
use capybara::{PacketToMaster, PacketToSlave};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log_panics::init();

    let (set_raw_angle_tx, set_raw_angle_rx) = mpsc::channel::<f64>(1);
    let (angle_tx, angle_rx) = watch::channel(0.0);
    let (camera_tx, camera_rx) = watch::channel(RgbImage::new(640, 480));
    let (photo_request_tx, photo_request_rx) = mpsc::channel(1);
    let (button_tx, _) = broadcast::channel(1);
    let (photo_data_tx, mut photo_data_rx) = broadcast::channel(4);
    let (encoder_tx, mut encoder_rx) = broadcast::channel(32);

    let (radio_up_tx_video, radio_up_rx) = broadcast::channel(32);
    let radio_up_tx_photo = radio_up_tx_video.clone();
    let radio_up_tx_odometry = radio_up_tx_video.clone();

    let (radio_down_tx, mut radio_down_rx) = broadcast::channel(32);

    let (odometry_tx, mut odometry_rx) = watch::channel(Odometry {
        x: 0.0,
        y: 0.0,
        theta: 0.0,
    });
    let (velocity_tx, velocity_rx) = watch::channel(Velocity {
        linear: 0.0,
        angular: 0.0,
    });

    let mut tasks = JoinSet::<Result<()>>::new();
    tasks.spawn(ros::run_ros(odometry_tx, velocity_rx));
    tasks.spawn(muskrat::run_muskrat(set_raw_angle_rx, button_tx));
    tasks.spawn(servo::run_servo(angle_rx, set_raw_angle_tx));
    tasks.spawn(ws::run_ws(radio_up_tx_video.clone(), radio_down_tx.clone()));
    tasks.spawn(radio::run_radio(radio_up_rx, radio_down_tx));
    tasks.spawn(encoder::run_encoder(camera_rx, encoder_tx));
    tasks.spawn(phototaker::run_phototaker(
        photo_request_rx,
        camera_tx.subscribe(),
        photo_data_tx,
    ));
    tasks.spawn(camera::run_camera(camera_tx));

    tasks.spawn(async move {
        loop {
            let cmd_bytes = match radio_down_rx.recv().await {
                Ok(d) => d,
                Err(broadcast::error::RecvError::Lagged(l)) => {
                    error!("lagged for {l} packets");
                    continue;
                }
                Err(_) => return Ok(()),
            };
            debug!("got cmd len = {}", cmd_bytes.len());
            let cmd = PacketToSlave::try_from_slice(&cmd_bytes)?;
            match cmd {
                PacketToSlave::TakePhoto => {
                    let _ = photo_request_tx.send(()).await;
                }
                PacketToSlave::SetVelocity(v) => {
                    let _ = velocity_tx.send(v);
                }
                PacketToSlave::SetAngle(a) => {
                    let _ = angle_tx.send(a);
                }
            }
        }
    });

    tasks.spawn(async move {
        let mut skipped = 4u8;
        while odometry_rx.changed().await.is_ok() {
            if skipped > 3 {
                skipped = 0;
                let o = (*odometry_rx.borrow()).clone();
                let pkt = PacketToMaster::Odometry(o);
                let _ = radio_up_tx_odometry.send(pkt.try_to_vec()?);
            } else {
                skipped += 1;
            }
        }
        Ok(())
    });
    tasks.spawn(async move {
        loop {
            let video_data = match encoder_rx.recv().await {
                Ok(d) => d,
                Err(broadcast::error::RecvError::Lagged(l)) => {
                    error!("lagged for {l} video packets");
                    continue;
                }
                Err(_) => return Ok(()),
            };
            let pkt = PacketToMaster::Video(video_data);
            let _ = radio_up_tx_video.send(pkt.try_to_vec()?);
        }
    });
    tasks.spawn(async move {
        loop {
            let photo_data = match photo_data_rx.recv().await {
                Ok(d) => d,
                Err(broadcast::error::RecvError::Lagged(l)) => {
                    error!("lagged for {l} photos");
                    continue;
                }
                Err(_) => return Ok(()),
            };
            let pkt = PacketToMaster::Photo(photo_data);
            let _ = radio_up_tx_photo.send(pkt.try_to_vec()?);
        }
    });

    wait_tasks(tasks).await;
    Ok(())
}
