use anyhow::Result;
use tokio::spawn;
use tokio::sync::broadcast;

use capybara::radio;
use capybara::ws;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log_panics::init();

    let (up_tx, _) = broadcast::channel(32);
    let (down_tx, down_rx) = broadcast::channel(32);

    let ws_task = spawn(ws::run_ws(up_tx.clone(), down_tx));
    let radio_task = spawn(radio::run_radio(down_rx, up_tx));

    radio_task.await??;
    ws_task.await??;

    Ok(())
}
