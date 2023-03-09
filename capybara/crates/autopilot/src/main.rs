use anyhow::Result;
use image::RgbImage;
use itertools::Itertools;
use log::*;
use opencv::{core, imgproc, prelude::*, types};
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::spawn_blocking;
use tokio::task::JoinSet;

use camera::run_camera;
use common::init_log;
use common::wait_tasks;
use muskrat::run_muskrat;
use muskrat::servo::run_servo;
use proto::Odometry;
use rc::run_rc;
use ros::run_ros;
use ws::run_ws;

#[tokio::main]
async fn main() -> Result<()> {
    init_log();

    let (set_raw_angle_tx, set_raw_angle_rx) = mpsc::channel::<f64>(1);
    let (angle_tx, angle_rx) = watch::channel(2390.0);
    let (camera_tx, mut camera_rx) = watch::channel(RgbImage::new(640, 480));
    let (button_tx, mut button_rx) = broadcast::channel(1);

    let (up_tx, _) = broadcast::channel(32);
    let (down_tx, down_rx) = broadcast::channel(32);

    let (odometry_tx, mut odometry_rx) = watch::channel(Odometry {
        x: 0.0,
        y: 0.0,
        theta: 0.0,
    });
    let (velocity_tx, velocity_rx) = broadcast::channel(1);

    let mut tasks = JoinSet::<Result<()>>::new();
    tasks.spawn(run_muskrat(set_raw_angle_rx, button_tx));
    tasks.spawn(run_servo(angle_rx, set_raw_angle_tx));
    tasks.spawn(run_ws(up_tx.clone(), down_tx));
    tasks.spawn(run_camera(camera_tx));
    tasks.spawn(run_rc(
        down_rx,
        up_tx,
        angle_tx,
        velocity_tx.clone(),
        odometry_tx.subscribe(),
        camera_rx.clone(),
    ));
    tasks.spawn(run_ros(odometry_tx, velocity_rx));

    tasks.spawn(async move {
        use AutopilotStage::*;
        info!("Started autopilot");

        let _ = button_rx.recv().await;
        info!("button pressed");
        let mut stage = Init;
        while camera_rx.changed().await.is_ok() {
            info!("Autopilot image received");
            let img = (*camera_rx.borrow()).clone();
            let (w, h) = (img.width() as i32, img.height() as i32);
            let img_vec = img.as_raw().clone();
            let mut vx = 0.0f32;
            let mut vy = 0.0f32;
            let mut b = false;
            let circle = spawn_blocking(move || get_circle_pos(h, w, img_vec)).await?;
            if let Ok(circle) = circle {
                info!("Circle found: {:?}", circle);
                if let Some(Circle { x }) = circle {
                    match stage {
                        Init => {
                            stage = Driving;
                        }
                        Driving => {
                            vy = (x - 0.8) / 5.0;
                            vx = 0.01;
                            b = false;
                        }
                        _ => {}
                    }
                } else {
                    match stage {
                        Init => {
                            // vy = 0.05;
                            b = false;
                        }
                        Driving => {
                            vy = 0.0;
                            vx = 0.0;
                            stage = Blindly;
                            b = true;
                        }
                        Blindly => {}
                    }
                }
            }
            info!("{vx} {vy} {stage:?}");
            if b {
                _ = common::drive_distance(0.26, &mut odometry_rx, &velocity_tx).await;
            } else {
                _ = velocity_tx.send(proto::Velocity {
                    linear: vx as f64,
                    angular: vy as f64,
                });
            }
        }
        info!("Exited autopilot");
        Ok(())
    });

    wait_tasks(tasks).await;
    Ok(())
}

#[derive(Debug)]
struct Circle {
    x: f32,
}

#[derive(Default, Clone, Debug)]
enum AutopilotStage {
    #[default]
    Init,
    Driving,
    Blindly,
}

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
    imgproc::cvt_color(&blurred, &mut hsv, imgproc::COLOR_RGB2HSV, 0)?;
    let mut a = Mat::default();
    core::in_range(
        &hsv,
        &core::Scalar::new(15.0, 50.0, 50.0, 0.0), // TODO: color
        &core::Scalar::new(80.0, 255.0, 255.0, 0.0),
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
    debug!("{:?}", contours);
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
        debug!("{x} {y} {r}");
        // info!("{} {} {}", x, y, r);
        let mut message = None;
        if r > 0.05 {
            message = Some(Circle { x });
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
