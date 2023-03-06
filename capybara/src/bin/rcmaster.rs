use anyhow::Result;
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use futures::SinkExt;
use futures::StreamExt;
use log::*;
use tokio::sync::broadcast;
use tokio::task::JoinSet;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use capybara::decoder;
use capybara::photosaver;
use capybara::wait_tasks;
use capybara::{PacketToMaster, PacketToSlave};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log_panics::init();

    let ws_stream = match connect_async("ws://10.8.0.2:8264").await {
        Ok((stream, _)) => stream,
        Err(_) => {
            return Ok(());
        }
    };
    let (mut sender, mut receiver) = ws_stream.split();

    let (encoder_tx, encoder_rx) = broadcast::channel(16);
    let (photo_data_tx, photo_data_rx) = broadcast::channel(32);
    let (image_tx, mut image_rx) = broadcast::channel(1);

    let mut tasks = JoinSet::<Result<()>>::new();
    tasks.spawn(decoder::run_decoder(encoder_rx, image_tx));
    tasks.spawn(photosaver::run_photosaver(photo_data_rx));

    tasks.spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let pkt = PacketToSlave::TakePhoto;
            let msg = Message::Binary(pkt.try_to_vec()?);
            if sender.send(msg).await.is_err() {
                return Ok(());
            };
        }
    });
    tasks.spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Binary(b) => {
                    let cmd = PacketToMaster::try_from_slice(&b)?;
                    match cmd {
                        PacketToMaster::Video(vd) => {
                            let _ = encoder_tx.send(vd);
                        }
                        PacketToMaster::Photo(pd) => {
                            let _ = photo_data_tx.send(pd);
                        }
                        PacketToMaster::Odometry(o) => {
                            info!("got odometry x = {}, y = {}, theta = {}", o.x, o.y, o.theta);
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
        Ok(())
    });
    tasks.spawn(async move {
        loop {
            let img = match image_rx.recv().await {
                Ok(i) => i,
                Err(broadcast::error::RecvError::Lagged(l)) => {
                    error!("lagged for {l} frames");
                    continue;
                }
                Err(_) => return Ok(()),
            };
            debug!("got image {}", img.as_raw().len());
        }
    });

    wait_tasks(tasks).await;
    Ok(())
}
