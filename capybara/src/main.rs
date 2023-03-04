use futures::future::join_all;
use tokio::spawn;
use tokio::sync::{broadcast, mpsc};

mod camera;
mod decoder;
mod encoder;
mod muskrat;
mod radio;

#[tokio::main]
async fn main() {
    let (_, set_angle_rx) = mpsc::channel::<u8>(1);
    let muskrat_task = spawn(muskrat::run_muskrat(set_angle_rx));

    let (camera_tx, camera_rx) = broadcast::channel(1);
    let camera_task = spawn(camera::run_camera(camera_tx));

    let (encoder_tx, encoder_rx) = broadcast::channel(256);
    let encoder_rx_2 = encoder_tx.subscribe();
    let encoder_task = spawn(encoder::run_encoder(camera_rx, encoder_tx));

    let radio_task = spawn(radio::run_radio(encoder_rx));
    let decoder_task = spawn(decoder::run_decoder(encoder_rx_2));

    join_all(vec![
        muskrat_task,
        camera_task,
        encoder_task,
        radio_task,
        decoder_task,
    ])
    .await;
}
