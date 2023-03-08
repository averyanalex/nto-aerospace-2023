use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use image::RgbImage;
use itertools::Itertools;
use log::*;
use opencv::{core, features2d, highgui, imgproc, prelude::*, types, videoio};
use std::sync::{Arc, Mutex};
use tokio::sync::watch::Sender;
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::spawn_blocking;
use tokio::task::JoinSet;

use camera::run_camera;
use common::wait_tasks;
// use ws::run_ws;
use proto::{Odometry, Velocity};
use proto::{PacketToMaster, PacketToSlave};
// use encoder::run_encoder;
use muskrat::run_muskrat;
// use muskrat::servo::run_servo;
// use phototaker::run_phototaker;
// use radio::run_radio;
use common::init_log;
use ros::run_ros;

#[tokio::main]
async fn main() -> Result<()> {
    init_log();

    let (set_raw_angle_tx, set_raw_angle_rx) = mpsc::channel::<f64>(1);
    // let (angle_tx, angle_rx) = watch::channel(0.0);
    let (camera_tx, mut camera_rx) = watch::channel(RgbImage::new(640, 480));
    // let (photo_request_tx, photo_request_rx) = mpsc::channel(1);
    let (button_tx, mut button_rx) = broadcast::channel(1);
    // let (photo_data_tx, mut photo_data_rx) = broadcast::channel(4);
    // let (encoder_tx, mut encoder_rx) = broadcast::channel(32);

    // let (radio_up_tx_video, radio_up_rx) = broadcast::channel(32);
    // let radio_up_tx_photo = radio_up_tx_video.clone();
    // let radio_up_tx_odometry = radio_up_tx_video.clone();

    // let (radio_down_tx, mut radio_down_rx) = broadcast::channel(32);

    let (odometry_tx, mut odometry_rx) = watch::channel(Odometry {
        x: 0.0,
        y: 0.0,
        theta: 0.0,
    });
    let (velocity_tx, velocity_rx) = watch::channel(Velocity {
        linear: 0.0,
        angular: 0.0,
    });

    let mut tasks = JoinSet::<Result<()>>::new();
    tasks.spawn(run_ros(odometry_tx, velocity_rx));
    tasks.spawn(run_muskrat(set_raw_angle_rx, button_tx));
    // tasks.spawn(run_servo(angle_rx, set_raw_angle_tx));
    // tasks.spawn(run_ws(radio_up_tx_video.clone(), radio_down_tx.clone()));
    // tasks.spawn(run_radio(
    //     "/dev/serial/by-path/platform-fd500000.pcie-pci-0000:01:00.0-usb-0:1.2:1.0-port0",
    //     radio_up_rx,
    //     radio_down_tx,
    // ));
    // tasks.spawn(run_encoder(camera_rx, encoder_tx));
    // tasks.spawn(run_phototaker(
    //     photo_request_rx,
    //     camera_tx.subscribe(),
    //     photo_data_tx,
    // ));
    tasks.spawn(run_camera(camera_tx));

    // tasks.spawn(async move {
    //     loop {
    //         let cmd_bytes = match radio_down_rx.recv().await {
    //             Ok(d) => d,
    //             Err(broadcast::error::RecvError::Lagged(l)) => {
    //                 error!("lagged for {l} packets");
    //                 continue;
    //             }
    //             Err(_) => return Ok(()),
    //         };
    //         debug!("got cmd len = {}", cmd_bytes.len());
    //         let cmd = PacketToSlave::try_from_slice(&cmd_bytes)?;
    //         match cmd {
    //             PacketToSlave::TakePhoto => {
    //                 let _ = photo_request_tx.send(()).await;
    //             }
    //             PacketToSlave::SetVelocity(v) => {
    //                 let _ = velocity_tx.send(v);
    //             }
    //             PacketToSlave::SetAngle(a) => {
    //                 let _ = angle_tx.send(a);
    //             }
    //         }
    //     }
    // });

    // tasks.spawn(async move {
    //     let mut skipped = 4u8;
    //     while odometry_rx.changed().await.is_ok() {
    //         if skipped > 3 {
    //             skipped = 0;
    //             let o = (*odometry_rx.borrow()).clone();
    //             let pkt = PacketToMaster::Odometry(o);
    //             let _ = radio_up_tx_odometry.send(pkt.try_to_vec()?);
    //         } else {
    //             skipped += 1;
    //         }
    //     }
    //     Ok(())
    // });
    // tasks.spawn(async move {
    //     loop {
    //         let video_data = match encoder_rx.recv().await {
    //             Ok(d) => d,
    //             Err(broadcast::error::RecvError::Lagged(l)) => {
    //                 error!("lagged for {l} video packets");
    //                 continue;
    //             }
    //             Err(_) => return Ok(()),
    //         };
    //         let pkt = PacketToMaster::Video(video_data);
    //         let _ = radio_up_tx_video.send(pkt.try_to_vec()?);
    //     }
    // });
    // tasks.spawn(async move {
    //     loop {
    //         let photo_data = match photo_data_rx.recv().await {
    //             Ok(d) => d,
    //             Err(broadcast::error::RecvError::Lagged(l)) => {
    //                 error!("lagged for {l} photos");
    //                 continue;
    //             }
    //             Err(_) => return Ok(()),
    //         };
    //         let pkt = PacketToMaster::Photo(photo_data);
    //         let _ = radio_up_tx_photo.send(pkt.try_to_vec()?);
    //     }
    // });

    let _ = button_rx.recv().await;
    info!("button pressed");

    tasks.spawn(async move {
        use AutopilotStage::*;
        let mut stage = Arc::new(Mutex::new(Init));
        while camera_rx.changed().await.is_ok() {
            let img = (*camera_rx.borrow()).clone();
            let (w, h) = (img.width() as i32, img.height() as i32);
            let img_vec = img.as_raw().clone();
            let stage_clone = stage.clone();
            let (vx, vy, b) = spawn_blocking(move || {
                let mut vx = 0.;
                let mut vy = 0.;
                let circle = get_circle_pos(h, w, img_vec);
                if let Ok(circle) = circle {
                    if let Some(Circle { x, y, r }) = circle {
                        match *stage_clone.lock().unwrap() {
                            Init => {
                                let mut st = stage_clone.lock().unwrap();
                                *st = Driving
                            }
                            Driving => {
                                vy = (x - 0.8) / 5.0;
                                vx = 0.01;
                                return (vx, vy, false);
                            }
                            _ => {}
                        }
                    } else {
                        match *stage_clone.lock().unwrap() {
                            Init => {
                                vy = 0.05;
                                return (vx, vy, false)
                            }
                            Driving => {
                                vy = 0.0;
                                vx = 0.0;
                                let mut st = stage_clone.lock().unwrap();
                                *st = Blindly;
                                return (vx, vy, true);
                            }
                            _ => {}
                        }
                    }
                }
                (0.0, 0.0, false)
            })
            .await?;
            if b {
                _ = common::drive_distance(0.26, &mut odometry_rx, &velocity_tx);
            } else {
                _ = velocity_tx.send(proto::Velocity {
                    linear: vx as f64,
                    angular: vy as f64,
                });
            }
        }
        Ok(())
    });

    wait_tasks(tasks).await;
    Ok(())
}

struct Circle {
    x: f32,
    y: f32,
    r: f32,
}

#[derive(Default)]
enum AutopilotStage {
    #[default]
    Init,
    Driving,
    Blindly,
    Off,
}

// pub async fn run_openballs(
//     camera_rx: watch::Receiver<RgbImage>,
//     balls_ts: broadcast::Sender<Option<Circle>>,
// ) -> Result<()> {
//     while cam_rx.changed().await.is_ok() {
//         let img = *camera_rx.borrow();
//         let (w, h) = (img.width(), img.height());
//         let img_vec = img.as_raw().clone();
//     }

//     spawn_blocking(move || {

//     });

//     Ok(())
// }

// // call on button press
// fn run_autopilot(camera_rx: watch::Receiver<RgbImage>) {

// }

fn get_circle_pos(h: i32, w: i32, mut vec: Vec<u8>) -> Result<Option<Circle>> {
    let frame = unsafe {
        Mat::new_rows_cols_with_data(
            h,
            w,
            core::Vec3b::opencv_type(), // FIXME: incorect type
            vec.as_mut_ptr() as *mut _,
            0,
        )
    }?;
    let mut blurred = Mat::default();
    imgproc::gaussian_blur(&frame, &mut blurred, core::Size::new(11, 11), 0., 0., 0)?;
    let mut hsv = Mat::default();
    imgproc::cvt_color(&frame, &mut hsv, imgproc::COLOR_RGB2HSV, 0)?;
    let mut a = Mat::default();
    core::in_range(
        &hsv,
        &core::Scalar::new(80.0, 200.0, 200.0, 0.0), // TODO: color
        &core::Scalar::new(120.0, 255.0, 255.0, 0.0),
        &mut a,
    )?;
    let mut b = Mat::default();
    imgproc::erode(
        &a,
        &mut b,
        &Mat::default(),
        core::Point::new(-1, -1),
        2,
        core::BORDER_CONSTANT,
        imgproc::morphology_default_border_value()?,
    )?;
    let mut c = Mat::default();
    imgproc::dilate(
        &b,
        &mut c,
        &Mat::default(),
        core::Point::new(-1, -1),
        2,
        core::BORDER_CONSTANT,
        imgproc::morphology_default_border_value()?,
    )?;
    let mut contours = types::VectorOfVectorOfPoint::new();
    imgproc::find_contours(
        &c,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
        core::Point::default(),
    )?;
    if !contours.is_empty() {
        let max = contours
            .iter()
            .filter_map(|x| {
                imgproc::contour_area(&x, false)
                    .ok()
                    .filter(|x| !x.is_nan())
            })
            .enumerate()
            .position_max_by(|x, y| x.1.total_cmp(&y.1))
            .unwrap(); // todo replace unwrap
        let maxcontour = contours.get(max).unwrap();
        let (mut center, mut radius) = (core::Point2f::default(), 0.);
        imgproc::min_enclosing_circle(&maxcontour, &mut center, &mut radius)?;
        // imgproc::moments
        let (x, y, r) = (center.x / w as f32, center.y / h as f32, radius / h as f32);
        // info!("{} {} {}", x, y, r);
        let mut message = None;
        if r > 0.05 {
            message = Some(Circle { x, y, r });
        }
        return Ok(message);
        // _ = tx1.send_timeout(message, Duration::from_millis(10));
        // imgproc::circle(
        //     &mut c,
        //     core::Point::new(center.x as i32, center.y as i32),
        //     radius as i32,
        //     core::Scalar::new(255.0, 0.0, 255.0, 0.0),
        //     3,
        //     imgproc::LINE_AA,
        //     0,
        // )?;
    } else {
        // _ = tx1.send_timeout(None, Duration::from_millis(10));
        return Ok(None);
    }
}
