use crate::yuvrgb::yuv_to_bgra;
use crossbeam::channel::unbounded;
use dav1d::{Decoder, Error::Again, PlanarImageComponent};
use futures::future::join_all;
use image::RgbImage;
use tokio::sync::broadcast;
use tokio::task::{spawn, spawn_blocking};

pub async fn run_decoder(
    mut data_rx: broadcast::Receiver<Vec<u8>>,
    image_tx: broadcast::Sender<RgbImage>,
) {
    let (pkt_tx, pkt_rx) = unbounded();
    let data_task = spawn(async move {
        loop {
            pkt_tx.send(data_rx.recv().await.unwrap()).unwrap();
        }
    });

    let decoder_task = spawn_blocking(move || {
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

            let rgb_buf = yuv_to_bgra(&src_buf, strides);

            let image = RgbImage::from_raw(640, 480, rgb_buf).unwrap();
            image_tx.send(image).unwrap();
        };
        let mut decoder_settings = dav1d::Settings::new();
        decoder_settings.set_max_frame_delay(1);
        let mut decoder = Decoder::with_settings(&decoder_settings).unwrap();
        loop {
            let data = pkt_rx.recv().unwrap();

            while let Err(send_data_err) = decoder.send_data(data.clone(), None, None, None) {
                match send_data_err {
                    Again => {
                        while let Err(send_pending_err) = decoder.send_pending_data() {
                            match send_pending_err {
                                Again => match decoder.get_picture() {
                                    Ok(picture) => {
                                        handle_picture(picture);
                                    }
                                    Err(_) => {}
                                },
                                _ => panic!("{}", send_pending_err),
                            }
                        }
                    }
                    _ => panic!("{}", send_data_err),
                }
            }
        }
    });

    join_all(vec![data_task, decoder_task]).await;
}
