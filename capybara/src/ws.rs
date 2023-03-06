use anyhow::Result;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
    Extension, Router,
};
use futures::{StreamExt, SinkExt};
use log::*;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::task::spawn;
use tokio::{sync::broadcast, task::JoinHandle};

struct ChannelsSpawner {
    up_tx: broadcast::Sender<Vec<u8>>,
    down_tx: broadcast::Sender<Vec<u8>>,
}

impl ChannelsSpawner {
    pub fn new(up_tx: broadcast::Sender<Vec<u8>>, down_tx: broadcast::Sender<Vec<u8>>) -> Self {
        Self { up_tx, down_tx }
    }

    pub fn get_up_rx(&self) -> broadcast::Receiver<Vec<u8>> {
        self.up_tx.subscribe()
    }

    pub fn get_down_tx(&self) -> broadcast::Sender<Vec<u8>> {
        self.down_tx.clone()
    }
}

pub async fn run_ws(
    up_tx: broadcast::Sender<Vec<u8>>,
    down_tx: broadcast::Sender<Vec<u8>>,
) -> Result<()> {
    let channels_spawner = Arc::new(ChannelsSpawner::new(up_tx, down_tx));

    let app = Router::new()
        .route("/", get(ws_handler))
        .layer(Extension(channels_spawner));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8264));
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await?;

    Ok(())
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    channels_spawner: Extension<Arc<ChannelsSpawner>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, channels_spawner))
}

async fn handle_socket(socket: WebSocket, channels_spawner: Extension<Arc<ChannelsSpawner>>) {
    let mut up_rx = channels_spawner.get_up_rx();
    let down_tx = channels_spawner.get_down_tx();

    let (mut sender, mut receiver) = socket.split();

    let reader_task: JoinHandle<Result<()>> = spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => return Ok(()),
                Message::Binary(bin) => {
                    debug!("got from ws len = {}", bin.len());
                    if down_tx.send(bin).is_err() {
                        return Ok(());
                    };
                }
                Message::Text(t) => {
                    info!("got message: {}", t);
                }
                _ => return Ok(()),
            }
        }
        Ok(())
    });
    let writer_task: JoinHandle<Result<()>> = spawn(async move {
        loop {
            let data = match up_rx.recv().await {
                Ok(d) => d,
                Err(broadcast::error::RecvError::Lagged(l)) => {
                    error!("lagged {}", l);
                    continue;
                }
                Err(_) => return Ok(()),
            };
            debug!("sending {} bytes to ws", data.len());
            if sender.send(Message::Binary(data)).await.is_err() {
                return Ok(());
            };
        }
    });

    reader_task.await.unwrap().unwrap();
    writer_task.await.unwrap().unwrap();
}
