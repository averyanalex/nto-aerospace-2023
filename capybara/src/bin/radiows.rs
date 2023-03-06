use anyhow::Result;
use tokio::sync::broadcast;

use capybara::radio;
use capybara::wait_tasks;
use capybara::ws;
use tokio::task::JoinSet;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log_panics::init();

    let (up_tx, _) = broadcast::channel(32);
    let (down_tx, down_rx) = broadcast::channel(32);

    let mut tasks = JoinSet::<Result<()>>::new();
    tasks.spawn(ws::run_ws(up_tx.clone(), down_tx));
    tasks.spawn(radio::run_radio(down_rx, up_tx));

    wait_tasks(tasks).await;
}
