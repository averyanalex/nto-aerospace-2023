use tokio::sync::{mpsc, watch};
use tokio::time::{sleep, Duration, Instant};

const MAX_SPEED: f64 = 10.0;
const ACCEL: f64 = 2.0;

pub async fn run_servo(
    set_angle_rx: watch::Receiver<f64>,
    set_raw_angle_tx: mpsc::Sender<f64>,
) {
    let mut angle: f64 = 10.0;
    let mut speed: f64 = 0.0;
    let mut last_run = Instant::now();

    loop {
        let target = *set_angle_rx.borrow();
        let dt = (Instant::now() - last_run).as_secs_f64();
        last_run = Instant::now();
        if angle < target {
            speed = if angle + (ACCEL * (speed / ACCEL).powi(2) / 2.0) < target {
                if speed + ACCEL * dt < MAX_SPEED {
                    speed + ACCEL * dt
                } else {
                    speed - ACCEL * dt
                }
            } else {
                if speed - ACCEL * dt > -MAX_SPEED {
                    speed - ACCEL * dt
                } else {
                    speed + ACCEL * dt
                }
            };
        } else if (target - angle).abs() < 0.1 {
            while speed > 0.0 {
                speed - ACCEL * dt;
            }
        } else {
            speed = if angle - (ACCEL * (speed / ACCEL).powi(2) / 2.0) < target {
                if speed + ACCEL * dt < MAX_SPEED {
                    speed + ACCEL * dt
                } else {
                    speed - ACCEL * dt
                }
            } else {
                if speed - ACCEL * dt > -MAX_SPEED {
                    speed - ACCEL * dt
                } else {
                    speed + ACCEL * dt
                }
            };
        }

        angle += speed * dt;
        set_raw_angle_tx.send(angle).await.unwrap();
        sleep(Duration::from_millis(10)).await;
    }

    // while let Some(angle) = set_angle_rx.recv().await {
    //     set_raw_angle_tx.send(angle).await.unwrap();
    // }
}
