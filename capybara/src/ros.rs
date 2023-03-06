use crate::{Odometry, Velocity};
use anyhow::Result;
use tokio::sync::watch;
use tokio::task::{spawn_blocking, JoinHandle};

pub async fn run_ros(
    odometry_tx: watch::Sender<Odometry>,
    velocity_rx: watch::Receiver<Velocity>,
) -> Result<()> {
    rosrust::init("capybara");

    let velocity_pub = rosrust::publish("cmd_vel", 1).unwrap();
    let velocity_rate = rosrust::rate(30.0);

    let _ = rosrust::subscribe(
        "odom_pose2d",
        1,
        move |v: rosrust_msg::geometry_msgs::Pose2D| {
            let _ = odometry_tx.send(Odometry {
                x: v.x,
                y: v.y,
                theta: v.theta,
            });
        },
    )
    .unwrap();

    let velocity_task: JoinHandle<Result<(), rosrust::error::Error>> = spawn_blocking(move || {
        while rosrust::is_ok() {
            let velocity = (*velocity_rx.borrow()).clone();
            let velocity_msg = rosrust_msg::geometry_msgs::Twist {
                linear: rosrust_msg::geometry_msgs::Vector3 {
                    x: 0.0,
                    y: 0.0,
                    z: velocity.angular,
                },
                angular: rosrust_msg::geometry_msgs::Vector3 {
                    x: velocity.linear,
                    y: 0.0,
                    z: 0.0,
                },
            };
            velocity_pub.send(velocity_msg)?;
            velocity_rate.sleep();
        }
        Ok(())
    });

    velocity_task.await?.unwrap();

    Ok(())
}
