use crate::yuvrgb::yuv_to_bgra;
use anyhow::{bail, Result};
use crossbeam::channel::unbounded;
use dav1d::{Decoder, Error::Again, Error::InvalidArgument, PlanarImageComponent};
use futures::future::join_all;
use image::RgbImage;
use log::*;
use tokio::sync::broadcast;
use tokio::task::{spawn, spawn_blocking, JoinHandle};

pub async fn run_decoder(
    mut data_rx: broadcast::Receiver<Vec<u8>>,
    image_tx: broadcast::Sender<RgbImage>,
) -> Result<()> {
    let (pkt_tx, pkt_rx) = unbounded();
    let data_task = spawn(async move {
        loop {
            let data = match data_rx.recv().await {
                Ok(d) => d,
                Err(broadcast::error::RecvError::Lagged(l)) => {
                    error!("decoder lagged for {l} packets");
                    continue;
                }
                Err(_) => return Ok(()),
            };
            if pkt_tx.send(data).is_err() {
                return Ok(());
            };
        }
    });

    let decoder_task: JoinHandle<Result<()>> = spawn_blocking(move || {
        let handle_picture = |picture: dav1d::Picture| {
            let planes = &[
                picture.plane(PlanarImageComponent::Y),
                picture.plane(PlanarImageComponent::U),
                picture.plane(PlanarImageComponent::V),
            ];

            let src_buf = planes.iter().map(AsRef::as_ref).collect::<Vec<_>>();
            let strides = &[
                picture.stride(PlanarImageComponent::Y) as usize,
                picture.stride(PlanarImageComponent::U) as usize,
                picture.stride(PlanarImageComponent::V) as usize,
            ];

            let rgb_buf = yuv_to_bgra(&src_buf, strides)?;

            let image = match RgbImage::from_raw(640, 480, rgb_buf) {
                Some(i) => i,
                None => bail!("image container too small"),
            };
            if image_tx.send(image).is_err() {
                return Ok(());
            };
            Ok(())
        };
        let mut decoder_settings = dav1d::Settings::new();
        decoder_settings.set_max_frame_delay(1);

        let mut decoder = Decoder::with_settings(&decoder_settings)?;
        loop {
            let data = match pkt_rx.recv() {
                Ok(d) => d,
                Err(_) => return Ok(()),
            };

            while let Err(send_data_err) = decoder.send_data(data.clone(), None, None, None) {
                match send_data_err {
                    Again => {
                        while let Err(send_pending_err) = decoder.send_pending_data() {
                            match send_pending_err {
                                Again => match decoder.get_picture() {
                                    Ok(picture) => {
                                        handle_picture(picture)?;
                                    }
                                    Err(_) => {}
                                },
                                _ => bail!("{}", send_pending_err),
                            }
                        }
                    }
                    InvalidArgument => {
                        warn!("encoder InvalidArgument");
                        break;
                    }
                    _ => bail!("{}", send_data_err),
                }
            }
        }
    });

    join_all(vec![data_task, decoder_task]).await;
    Ok(())
}
