use anyhow::{bail, Error, Result};
use crossbeam::channel::unbounded;
use futures::future::join_all;
use image::{Rgb, RgbImage};
use log::*;
use rav1e::config::SpeedSettings;
use rav1e::prelude::*;
use tokio::sync::{broadcast, watch};
use tokio::task::{spawn, spawn_blocking, JoinHandle};

pub async fn run_encoder(
    mut cam_rx: watch::Receiver<RgbImage>,
    data_tx: broadcast::Sender<Vec<u8>>,
) -> Result<()> {
    // Encoder configuration
    let mut enc = EncoderConfig::default();

    // Basic settings
    enc.time_base = Rational { num: 1, den: 5 }; // 5 FPS
    enc.width = 640;
    enc.height = 480;
    enc.chroma_sampling = ChromaSampling::Cs444;

    // Raspberry Pi
    enc.speed_settings = SpeedSettings::from_preset(10);
    enc.tiles = 4; // 4 cpu

    // Low birate
    enc.bitrate = 50;
    enc.min_quantizer = 180;

    // Low latency
    enc.speed_settings.rdo_lookahead_frames = 1;
    enc.low_latency = true;
    enc.max_key_frame_interval = 50;

    let cfg = Config::new().with_encoder_config(enc).with_threads(4);

    let (frame_tx, frame_rx) = unbounded();
    let frame_task: JoinHandle<Result<(), Error>> = spawn(async move {
        let mut send_this_time = true;
        while cam_rx.changed().await.is_ok() {
            let img = (*cam_rx.borrow()).clone();
            if send_this_time {
                if frame_tx.send(img).is_err() {
                    break;
                }
                send_this_time = false;
            } else {
                send_this_time = true;
            }
        }
        Ok(())
    });

    let encoder_task: JoinHandle<Result<()>> = spawn_blocking(move || {
        let mut ctx: Context<u8> = cfg.new_context()?;
        loop {
            // Drop old frames and receive one
            while frame_rx.len() > 2 {
                warn!("dropping frames");
                while frame_rx.len() > 1 {
                    if frame_rx.recv().is_err() {
                        return Ok(());
                    };
                }
            }
            let frame = match frame_rx.recv() {
                Ok(f) => f,
                Err(_) => return Ok(()),
            };

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
                dst.copy_from_raw_u8(&src, 640, 1);
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
                        bail!("unable to send frame");
                    }
                },
            }

            // Receive data from encoder
            match ctx.receive_packet() {
                Ok(pkt) => {
                    if data_tx.send(pkt.data).is_err() {
                        return Ok(());
                    };
                }
                Err(e) => match e {
                    EncoderStatus::LimitReached => {
                        warn!("read thread: Limit reached");
                    }
                    EncoderStatus::Encoded => debug!("read thread: Encoded"),
                    EncoderStatus::NeedMoreData => debug!("read thread: Need more data"),
                    _ => {
                        bail!("unable to receive packet");
                    }
                },
            };
        }
    });

    join_all(vec![frame_task, encoder_task]).await;
    Ok(())
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
