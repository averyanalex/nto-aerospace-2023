use anyhow::{Error, Result};
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use capybara::decoder;
use capybara::photosaver;
use capybara::{PacketToMaster, PacketToSlave};
use futures::SinkExt;
use futures::StreamExt;
use log::*;
use tokio::{spawn, sync::broadcast, task::JoinHandle};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log_panics::init();

    let ws_stream = match connect_async("ws://127.0.0.1:8264").await {
        Ok((stream, _)) => stream,
        Err(_) => {
            return Ok(());
        }
    };
    let (mut sender, mut receiver) = ws_stream.split();

    let (encoder_tx, encoder_rx) = broadcast::channel(16);
    let (photo_data_tx, photo_data_rx) = broadcast::channel(32);
    let (image_tx, mut image_rx) = broadcast::channel(1);

    let encoder_task = spawn(decoder::run_decoder(encoder_rx, image_tx));
    let photosaver_task = spawn(photosaver::run_photosaver(photo_data_rx));

    let send_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let pkt = PacketToSlave::TakePhoto;
            let msg = Message::Binary(pkt.try_to_vec()?);
            if sender.send(msg).await.is_err() {
                return Ok(());
            };
        }
    });
    let recv_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Binary(b) => {
                    let cmd = PacketToMaster::try_from_slice(&b)?;
                    match cmd {
                        PacketToMaster::VideoData(vd) => {
                            if encoder_tx.send(vd).is_err() {
                                return Ok(());
                            };
                        }
                        PacketToMaster::PhotoData(pd) => {
                            if photo_data_tx.send(pd).is_err() {
                                return Ok(());
                            };
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
        Ok(())
    });
    let image_task: JoinHandle<Result<()>> = tokio::spawn(async move {
        loop {
            let img = match image_rx.recv().await {
                Ok(i) => i,
                Err(broadcast::error::RecvError::Lagged(l)) => {
                    error!("decoder lagged for {l} packets");
                    continue;
                }
                Err(_) => return Ok(()),
            };
            debug!("got image {}", img.as_raw().len());
        }
    });

    recv_task.await??;
    send_task.await??;
    encoder_task.await??;
    photosaver_task.await??;
    image_task.await??;

    Ok(())
}
