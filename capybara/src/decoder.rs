use crossbeam::channel::unbounded;
use dav1d::Decoder;
use dav1d::Error::Again;
use dav1d::{PixelLayout, PlanarImageComponent};
use dcp::ImageFormat;
use dcv_color_primitives as dcp;
use futures::future::join_all;
use image::{Bgra, ImageBuffer, Rgb, RgbImage};
use tokio::sync::broadcast;
use tokio::task::{spawn, spawn_blocking};
// use yuv::{RGB, YUV};

pub async fn run_decoder(mut data_rx: broadcast::Receiver<Vec<u8>>) {
    let (pkt_tx, pkt_rx) = unbounded();
    let data_task = spawn(async move {
        loop {
            pkt_tx.send(data_rx.recv().await.unwrap()).unwrap();
        }
    });

    let decoder_task = spawn_blocking(move || {
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
                                        dcp::initialize();

                                        // let width = 640;
                                        // let height = 480;

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
                                        // let rgb_format = dcp::ImageFormat {
                                        //     pixel_format: dcp::PixelFormat::Rgb,
                                        //     color_space: dcp::ColorSpace::Lrgb,
                                        //     num_planes: 1,
                                        // };

                                        // let (width, height) = self.dimensions();
                                        let planes = &[
                                            picture.plane(PlanarImageComponent::Y),
                                            picture.plane(PlanarImageComponent::U),
                                            picture.plane(PlanarImageComponent::V),
                                        ];

                                        // let converter = yuv::convert::RGBConvert::<u8>::new(
                                        //     yuv::color::Range::Full,
                                        //     yuv::color::MatrixCoefficients::BT601,
                                        // )
                                        // .unwrap();

                                        // let mut rgb = [0u8; 640 * 480 * 3];
                                        // for i in 0..640 * 480 {
                                        //     let yuv_pix = yuv::YUV {
                                        //         y: planes[0][i],
                                        //         u: planes[1][i],
                                        //         v: planes[2][i],
                                        //     };
                                        //     let rgb_pix = converter.to_rgb(yuv_pix);
                                        //     rgb[i * 3 + 0] = rgb_pix.r;
                                        //     rgb[i * 3 + 1] = rgb_pix.g;
                                        //     rgb[i * 3 + 2] = rgb_pix.b;
                                        // }

                                        let src_buffers =
                                            planes.iter().map(AsRef::as_ref).collect::<Vec<_>>();
                                        // println!("{}", planes[0][0]);
                                        // println!("{}", planes[1][0]);
                                        // println!("{}", planes[2][0]);
                                        // println!("{}", planes[0][1]);
                                        // println!("{}", planes[1][1]);
                                        // println!("{}", planes[2][1]);
                                        // println!("{}", planes[0].len());
                                        let strides = &[
                                            picture.stride(PlanarImageComponent::Y) as usize,
                                            picture.stride(PlanarImageComponent::U) as usize,
                                            picture.stride(PlanarImageComponent::V) as usize,
                                        ];

                                        // println!("{:?}", src_buffers);

                                        let mut bgra_buf = [0u8; 640 * 480 * 4];
                                        let dst_bgra_buffers = &mut [&mut bgra_buf[..]];
                                        dcp::convert_image(
                                            640,
                                            480,
                                            &src_format,
                                            // Some(strides),
                                            None,
                                            &src_buffers,
                                            &bgra_format,
                                            None,
                                            dst_bgra_buffers,
                                        )
                                        .unwrap();

                                        // let mut rgb_v: Vec<_> = bgra_buf
                                        //     .chunks_exact(4)
                                        //     .flat_map(|pix| [pix[2], pix[1], pix[0]])
                                        //     .collect();
                                        // rgb_v.truncate(480 * 640 * 3);

                                        // let mut rgb = [0u8; 640 * 480 * 3];
                                        // for (src, dst) in
                                        //     bgra_buf.chunks_exact(4).zip(rgb.chunks_exact_mut(3))
                                        // {
                                        //     dst[0] = src[2];
                                        //     dst[1] = src[1];
                                        //     dst[2] = src[0];
                                        // }

                                        // println!("{:?}", bgra_buf);

                                        let img = ImageBuffer::<Bgra<u8>, Vec<u8>>::from_raw(
                                            640,
                                            480,
                                            bgra_buf.to_vec(),
                                        )
                                        .unwrap();
                                        img.save("sus.jpg").unwrap();

                                        // let src_bgra_buf = &(&[&bgra_buf[..]])[..];
                                        // let mut rgb_buf = [0u8; 640 * 480 * 3];
                                        // let dst_rgb_buffers = &mut [&mut rgb_buf[..]];
                                        // dcp::convert_image(
                                        //     width,
                                        //     height,
                                        //     &bgra_format,
                                        //     None,
                                        //     src_bgra_buf,
                                        //     &rgb_format,
                                        //     None,
                                        //     dst_rgb_buffers,
                                        // )
                                        // .unwrap();

                                        // let img = ImageBuffer::from_fn(width, height, |x, y| {
                                        //     let r =
                                        //     image::Rgb([0, 0, 0])
                                        // });
                                        // let mut rgb_buf = [0u8; 640 * 480 * 3];
                                        // let dst_rgb_buffers = &mut [&mut rgb_buf[..]];
                                        // dcp::convert_image(
                                        //     width,
                                        //     height,
                                        //     &src_format,
                                        //     Some(strides),
                                        //     &src_buffers,
                                        //     &dst_format,
                                        //     None,
                                        //     dst_bgra_buffers,
                                        // )
                                        // .unwrap();

                                        println!("Oh my pic!");
                                        // pic.plane(component)
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
