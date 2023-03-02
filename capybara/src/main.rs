use tokio::spawn;
use tokio::sync::mpsc;

mod muskrat;

#[tokio::main]
async fn main() {
    let (set_angle_tx, set_angle_rx) = mpsc::channel::<u8>(1);
    let musk = spawn(muskrat::run_muskrat(set_angle_rx));
    set_angle_tx.send(1).await.unwrap();
    musk.await.unwrap();
}
