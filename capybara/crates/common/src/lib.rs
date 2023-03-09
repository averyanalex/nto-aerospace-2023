use anyhow::bail;
use anyhow::Result;
use log::*;
use tokio::sync::watch;
use tokio::task::JoinSet;

use proto;

pub fn init_log() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log_panics::init();
}

pub async fn wait_tasks(mut tasks: JoinSet<Result<()>>) {
    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(_) => error!("task exited"),
            Err(e) => error!("task failed: {e}"),
        }
    }
}

pub async fn drive_distance(
    distance: f64,
    odometry_rx: &mut watch::Receiver<proto::Odometry>,
    velocity_tx: &watch::Sender<proto::Velocity>,
) -> Result<()> {
    debug!("Driving distance: {}", distance);
    let last_pos = (*odometry_rx.borrow()).clone();

    if velocity_tx
        .send(proto::Velocity {
            linear: 0.01,
            angular: 0.0,
        })
        .is_err()
    {
        bail!("");
    }

    while odometry_rx.changed().await.is_ok() {
        let pos = (*odometry_rx.borrow()).clone();
        if pos.distance_to(&last_pos) > distance - 0.005 {
            break;
        }
    }

    if velocity_tx
        .send(proto::Velocity {
            linear: 0.0,
            angular: 0.0,
        })
        .is_err()
    {
        bail!("");
    }
    Ok(())
}
