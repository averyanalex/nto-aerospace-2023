use crossbeam::channel::unbounded;
use futures::future::join_all;
use image::{Rgb, RgbImage};
use log::*;
use rav1e::config::SpeedSettings;
use rav1e::prelude::*;
use tokio::sync::broadcast;
use tokio::task::{spawn, spawn_blocking};

pub async fn run_encoder(
    mut cam_rx: broadcast::Receiver<RgbImage>,
    data_tx: broadcast::Sender<Vec<u8>>,
) {
    let mut enc = EncoderConfig::default();
    // Basic settings
    enc.time_base = Rational { num: 1, den: 10 };
    enc.width = 640;
    enc.height = 480;

    // Birate limit
    enc.bitrate = 40;
    enc.min_quantizer = 255;

    // AV1 magic
    enc.error_resilient = true;
    enc.speed_settings = SpeedSettings::from_preset(8);
    // enc.speed_settings.rdo_lookahead_frames = 1;
    // enc.min_key_frame_interval = 20;
    // enc.max_key_frame_interval = 50;
    // enc.low_latency = true;
    enc.still_picture = false;
    enc.tiles = 4;
    enc.chroma_sampling = ChromaSampling::Cs444;

    let cfg = Config::new().with_encoder_config(enc).with_threads(4);

    let (frame_tx, frame_rx) = unbounded::<RgbImage>();
    let frame_task = spawn(async move {
        loop {
            frame_tx.send(cam_rx.recv().await.unwrap()).unwrap();
        }
    });

    let encoder_task = spawn_blocking(move || {
        let mut ctx: Context<u8> = cfg.new_context().unwrap();
        loop {
            // Drop old frames and receive one
            while frame_rx.len() > 3 {
                warn!("dropping frame");
                frame_rx.recv().unwrap();
            }
            let frame = frame_rx.recv().unwrap();

            // Convert RgbImage to Frame
            let mut r_slice: Vec<u8> = vec![];
            let mut g_slice: Vec<u8> = vec![];
            let mut b_slice: Vec<u8> = vec![];
            for pixel in frame.pixels() {
                let (r, g, b) = to_ycbcr(pixel);
                r_slice.push(r);
                g_slice.push(g);
                b_slice.push(b);
            }
            let planes = vec![r_slice, g_slice, b_slice];
            let mut video_frame = ctx.new_frame();
            for (dst, src) in video_frame.planes.iter_mut().zip(planes) {
                dst.copy_from_raw_u8(&src, 480, 1);
            }

            // Send frame to encoder
            match ctx.send_frame(video_frame) {
                Ok(_) => {
                    debug!("queued frame");
                }
                Err(e) => match e {
                    EncoderStatus::EnoughData => {
                        warn!("unable to append frame to the internal queue");
                    }
                    _ => {
                        panic!("unable to send frame");
                    }
                },
            }

            // ctx.flush();

            // Receive data from encoder
            match ctx.receive_packet() {
                Ok(pkt) => {
                    data_tx.send(pkt.data).unwrap();
                }
                Err(e) => match e {
                    EncoderStatus::LimitReached => {
                        warn!("read thread: Limit reached");
                    }
                    EncoderStatus::Encoded => debug!("read thread: Encoded"),
                    EncoderStatus::NeedMoreData => debug!("read thread: Need more data"),
                    _ => {
                        warn!("read thread: Unable to receive packet");
                    }
                },
            }
        }
    });

    join_all(vec![frame_task, encoder_task]).await;
}

fn clamp(val: f32) -> u8 {
    (val.round() as u8).max(0_u8).min(255_u8)
}

fn to_ycbcr(pixel: &Rgb<u8>) -> (u8, u8, u8) {
    let [r, g, b] = pixel.0;

    let y = 16_f32 + (65.481 * r as f32 + 128.553 * g as f32 + 24.966 * b as f32) / 255_f32;
    let cb = 128_f32 + (-37.797 * r as f32 - 74.203 * g as f32 + 112.000 * b as f32) / 255_f32;
    let cr = 128_f32 + (112.000 * r as f32 - 93.786 * g as f32 - 18.214 * b as f32) / 255_f32;

    (clamp(y), clamp(cb), clamp(cr))
}
