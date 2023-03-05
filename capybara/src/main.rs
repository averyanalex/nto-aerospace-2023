use futures::future::join_all;
use tokio::spawn;
use tokio::sync::{broadcast, mpsc, watch};

mod camera;
mod decoder;
mod encoder;
mod muskrat;
mod radio;
mod servo;
mod yuvrgb;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    log_panics::init();

    let (set_raw_angle_tx, set_raw_angle_rx) = mpsc::channel(1);
    let muskrat_task = spawn(muskrat::run_muskrat(set_raw_angle_rx));

    let (_, set_angle_rx) = watch::channel(60.0);
    let servo_task = spawn(servo::run_servo(set_angle_rx, set_raw_angle_tx));

    // let (camera_tx, camera_rx) = broadcast::channel(1);
    // let camera_task = spawn(camera::run_camera(camera_tx));

    // let (encoder_tx, encoder_radio_rx) = broadcast::channel(32);
    // let encoder_decoder_rx = encoder_tx.subscribe();
    // let encoder_task = spawn(encoder::run_encoder(camera_rx, encoder_tx));

    // let radio_task = spawn(radio::run_radio(encoder_radio_rx));

    // let (decoded_image_tx, mut decoded_image_rx) = broadcast::channel(8);
    // let decoder_task = spawn(decoder::run_decoder(encoder_decoder_rx, decoded_image_tx));

    // let decoded_image_task = spawn(async move {
    //     loop {
    //         decoded_image_rx.recv().await.unwrap();
    //     }
    // });

    join_all(vec![
        muskrat_task,
        servo_task,
        // camera_task,
        // encoder_task,
        // radio_task,
        // decoder_task,
        // decoded_image_task,
    ])
    .await;
}
