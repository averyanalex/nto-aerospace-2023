use anyhow::Result;
use log::*;
use tokio::sync::broadcast;

pub async fn run_radio(
    mut up_rx: broadcast::Receiver<Vec<u8>>,
    _: broadcast::Sender<Vec<u8>>,
) -> Result<()> {
    loop {
        let data_to_send = match up_rx.recv().await {
            Ok(d) => d,
            Err(broadcast::error::RecvError::Lagged(l)) => {
                error!("lagged {}", l);
                continue;
            }
            Err(_) => return Ok(()),
        };
        debug!("sending {} bytes to radio", data_to_send.len());
        // if down_tx.send(data_to_send).is_err() {
        //     return Ok(());
        // };
    }
}
