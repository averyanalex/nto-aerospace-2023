use anyhow::Result;
use log::*;
use tokio::sync::{broadcast, mpsc, watch};

use capybara::muskrat;
use capybara::ros;
use capybara::wait_tasks;
use capybara::{Odometry, Velocity};
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log_panics::init();

    let (_, set_raw_angle_rx) = mpsc::channel::<f64>(1);
    let (button_tx, mut button_rx) = broadcast::channel(1);
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
    tasks.spawn(ros::run_ros(odometry_tx, velocity_rx));
    tasks.spawn(muskrat::run_muskrat(set_raw_angle_rx, button_tx));

    loop {
        match button_rx.recv().await {
            Err(broadcast::error::RecvError::Lagged(_)) => warn!("btn lagged"),
            Err(_) => break,
            Ok(_) => {}
        }

        info!("starting 1m trip");

        let last_pos = (*odometry_rx.borrow()).clone();

        if velocity_tx
            .send(Velocity {
                linear: 0.1,
                angular: 0.0,
            })
            .is_err()
        {
            break;
        }

        while odometry_rx.changed().await.is_ok() {
            let pos = (*odometry_rx.borrow()).clone();
            if pos.distance_to(&last_pos) > 1.0 {
                break;
            }
        }

        if velocity_tx
            .send(Velocity {
                linear: 0.0,
                angular: 0.0,
            })
            .is_err()
        {
            break;
        }
    }

    wait_tasks(tasks).await;
    Ok(())
}
