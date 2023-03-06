use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use image::RgbImage;
use log::*;
use tokio::spawn;
use tokio::sync::{broadcast, mpsc, watch};

use capybara::encoder;
use capybara::muskrat;
use capybara::phototaker;
use capybara::radio;
use capybara::{camera, ws};
use capybara::{PacketToMaster, PacketToSlave};
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log_panics::init();

    let (_, set_angle_rx) = mpsc::channel::<u8>(1);
    let (camera_tx, camera_rx) = watch::channel(RgbImage::new(640, 480));
    let (photo_request_tx, photo_request_rx) = mpsc::channel(1);
    let (photo_data_tx, mut photo_data_rx) = broadcast::channel(4);
    let (encoder_tx, mut encoder_rx) = broadcast::channel(32);

    let (radio_up_tx_video, radio_up_rx) = broadcast::channel(32);
    let radio_up_tx_photo = radio_up_tx_video.clone();

    let (radio_down_tx, mut radio_down_rx) = broadcast::channel(32);

    let muskrat_task = spawn(muskrat::run_muskrat(set_angle_rx));
    let ws_task = spawn(ws::run_ws(radio_up_tx_video.clone(), radio_down_tx.clone()));
    let radio_task = spawn(radio::run_radio(radio_up_rx, radio_down_tx));
    let encoder_task = spawn(encoder::run_encoder(camera_rx, encoder_tx));
    let phototaker_task = spawn(phototaker::run_phototaker(
        photo_request_rx,
        camera_tx.subscribe(),
        photo_data_tx,
    ));
    let camera_task = spawn(camera::run_camera(camera_tx));

    let command_handler_task: JoinHandle<Result<()>> = spawn(async move {
        loop {
            let cmd_bytes = match radio_down_rx.recv().await {
                Ok(d) => d,
                Err(broadcast::error::RecvError::Lagged(l)) => {
                    error!("lagged {}", l);
                    continue;
                }
                Err(_) => return Ok(()),
            };
            debug!("Got cmd len = {}", cmd_bytes.len());
            let cmd = PacketToSlave::try_from_slice(&cmd_bytes)?;
            match cmd {
                PacketToSlave::TakePhoto => {
                    if photo_request_tx.send(()).await.is_err() {
                        return Ok(());
                    }
                }
            }
        }
    });

    let video_sender_task: JoinHandle<Result<()>> = spawn(async move {
        loop {
            let video_data = match encoder_rx.recv().await {
                Ok(d) => d,
                Err(broadcast::error::RecvError::Lagged(l)) => {
                    error!("lagged {}", l);
                    continue;
                }
                Err(_) => return Ok(()),
            };
            let pkt = PacketToMaster::VideoData(video_data);
            if radio_up_tx_video.send(pkt.try_to_vec()?).is_err() {
                return Ok(());
            };
        }
    });
    let photo_sender_task: JoinHandle<Result<()>> = spawn(async move {
        loop {
            let photo_data = match photo_data_rx.recv().await {
                Ok(d) => d,
                Err(broadcast::error::RecvError::Lagged(l)) => {
                    error!("lagged {}", l);
                    continue;
                }
                Err(_) => return Ok(()),
            };
            let pkt = PacketToMaster::PhotoData(photo_data);
            if radio_up_tx_photo.send(pkt.try_to_vec()?).is_err() {
                return Ok(());
            };
        }
    });

    camera_task.await??;
    command_handler_task.await??;
    encoder_task.await??;
    muskrat_task.await??;
    photo_sender_task.await??;
    phototaker_task.await??;
    radio_task.await??;
    ws_task.await??;
    video_sender_task.await??;
    Ok(())
}