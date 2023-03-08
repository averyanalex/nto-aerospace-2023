use anyhow::Result;
use tokio::sync::broadcast;

use common::init_log;
use common::wait_tasks;
use radio::run_radio;
use tokio::task::JoinSet;
use ws::run_ws;

#[tokio::main]
async fn main() {
    init_log();

    let (up_tx, _) = broadcast::channel(32);
    let (down_tx, down_rx) = broadcast::channel(32);

    let mut tasks = JoinSet::<Result<()>>::new();
    tasks.spawn(run_ws(up_tx.clone(), down_tx));
    tasks.spawn(run_radio(
        "/dev/serial/by-path/platform-fd500000.pcie-pci-0000:01:00.0-usb-0:1.4:1.0-port0",
        down_rx,
        up_tx,
    ));

    wait_tasks(tasks).await;
}
