use anyhow::{bail, Result};
use proto::{Odometry, Velocity};
use tokio::sync::watch;
use tokio::task::spawn_blocking;

pub async fn run_ros(
    odometry_tx: watch::Sender<Odometry>,
    velocity_rx: watch::Receiver<Velocity>,
) -> Result<()> {
    match rosrust::try_init("capybara") {
        Ok(_) => {}
        Err(e) => bail!("failed to init ros: {e}"),
    }

    let velocity_pub = match rosrust::publish("cmd_vel", 1) {
        Ok(p) => p,
        Err(e) => bail!("can't create publisher to cmd_vel: {e}"),
    };
    let velocity_rate = rosrust::rate(10.0);

    let _odometry_subscriber = match rosrust::subscribe(
        "odom_pose2d",
        1,
        move |v: rosrust_msg::geometry_msgs::Pose2D| {
            let _ = odometry_tx.send(Odometry {
                x: v.x,
                y: v.y,
                theta: v.theta,
            });
        },
    ) {
        Ok(s) => s,
        Err(e) => bail!("can't create subscriber to odom_pose2d: {e}"),
    };

    spawn_blocking(move || {
        while rosrust::is_ok() {
            let velocity = (*velocity_rx.borrow()).clone();
            let velocity_msg = rosrust_msg::geometry_msgs::Twist {
                linear: rosrust_msg::geometry_msgs::Vector3 {
                    x: velocity.linear,
                    y: 0.0,
                    z: 0.0,
                },
                angular: rosrust_msg::geometry_msgs::Vector3 {
                    x: 0.0,
                    y: 0.0,
                    z: velocity.angular,
                },
            };
            match velocity_pub.send(velocity_msg) {
                Ok(_) => {}
                Err(e) => bail!("can't send velocity: {e}"),
            }
            velocity_rate.sleep();
        }
        Ok(())
    })
    .await?
}
