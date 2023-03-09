use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use image::RgbImage;
use log::*;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinSet;

use common::wait_tasks;
use encoder::run_encoder;
use phototaker::run_phototaker;
use proto::{Odometry, Velocity};
use proto::{PacketToMaster, PacketToSlave};

pub async fn run_rc(
    mut down_rx: broadcast::Receiver<Vec<u8>>,
    up_tx: broadcast::Sender<Vec<u8>>,
    angle_tx: watch::Sender<f64>,
    velocity_tx: broadcast::Sender<Velocity>,
    mut odometry_rx: watch::Receiver<Odometry>,
    camera_rx: watch::Receiver<RgbImage>,
) -> Result<()> {
    let mut tasks = JoinSet::<Result<()>>::new();

    let (photo_request_tx, photo_request_rx) = mpsc::channel(1);
    let (photo_data_tx, mut photo_data_rx) = broadcast::channel(8);
    tasks.spawn(run_phototaker(
        photo_request_rx,
        camera_rx.clone(),
        photo_data_tx,
    ));

    let (encoder_tx, mut encoder_rx) = broadcast::channel(32);
    tasks.spawn(run_encoder(camera_rx, encoder_tx));

    tasks.spawn(async move {
        loop {
            let cmd_bytes = match down_rx.recv().await {
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

    let up_tx_odometry = up_tx.clone();
    tasks.spawn(async move {
        let mut skipped = 4u8;
        while odometry_rx.changed().await.is_ok() {
            if skipped > 3 {
                skipped = 0;
                let o = (*odometry_rx.borrow()).clone();
                let pkt = PacketToMaster::Odometry(o);
                let _ = up_tx_odometry.send(pkt.try_to_vec()?);
            } else {
                skipped += 1;
            }
        }
        Ok(())
    });

    let up_tx_video = up_tx.clone();
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
            let _ = up_tx_video.send(pkt.try_to_vec()?);
        }
    });

    let up_tx_photo = up_tx.clone();
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
            let _ = up_tx_photo.send(pkt.try_to_vec()?);
        }
    });

    wait_tasks(tasks).await;

    Ok(())
}
