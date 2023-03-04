use crossbeam::channel::unbounded;
use dav1d::Decoder;
use dav1d::Error::Again;
use dav1d::PlanarImageComponent;
use dcv_color_primitives as dcp;
use futures::future::join_all;
use image::{Bgra, ImageBuffer};
use tokio::sync::broadcast;
use tokio::task::{spawn, spawn_blocking};

pub async fn run_decoder(mut data_rx: broadcast::Receiver<Vec<u8>>) {
    let (pkt_tx, pkt_rx) = unbounded();
    let data_task = spawn(async move {
        loop {
            pkt_tx.send(data_rx.recv().await.unwrap()).unwrap();
        }
    });
    dcp::initialize();

    let decoder_task = spawn_blocking(move || {
        let handle_picture = |picture: dav1d::Picture| {
            let src_format = dcp::ImageFormat {
                pixel_format: dcp::PixelFormat::I444,
                color_space: dcp::ColorSpace::Bt601,
                num_planes: 3,
            };
            let bgra_format = dcp::ImageFormat {
                pixel_format: dcp::PixelFormat::Bgra,
                color_space: dcp::ColorSpace::Rgb,
                num_planes: 1,
            };

            let planes = &[
                picture.plane(PlanarImageComponent::Y),
                picture.plane(PlanarImageComponent::U),
                picture.plane(PlanarImageComponent::V),
            ];

            let src_buffers = planes.iter().map(AsRef::as_ref).collect::<Vec<_>>();
            let strides = &[
                picture.stride(PlanarImageComponent::Y) as usize,
                picture.stride(PlanarImageComponent::U) as usize,
                picture.stride(PlanarImageComponent::V) as usize,
            ];

            let mut bgra_buf = [0u8; 640 * 480 * 4];
            let dst_bgra_buffers = &mut [&mut bgra_buf[..]];
            dcp::convert_image(
                640,
                480,
                &src_format,
                Some(strides),
                // None,
                &src_buffers,
                &bgra_format,
                None,
                dst_bgra_buffers,
            )
            .unwrap();

            let img =
                ImageBuffer::<Bgra<u8>, Vec<u8>>::from_raw(640, 480, bgra_buf.to_vec()).unwrap();
            img.save("sus.jpg").unwrap();
        };
        let mut decoder = Decoder::new().unwrap();
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
