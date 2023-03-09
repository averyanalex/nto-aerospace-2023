use anyhow::Result;
use bevy::{
    prelude::*,
    render::render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
};
use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use futures::SinkExt;
use futures::StreamExt;
use tokio::sync::{
    broadcast,
    mpsc::{error::TryRecvError, Receiver, Sender},
};
use tokio::task::JoinSet;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use decoder::run_decoder;
use photosaver::run_photosaver;
use proto::Velocity;
use proto::{PacketToMaster, PacketToSlave};

use common::{VIDEO_HEIGHT, VIDEO_WIDTH};

#[tokio::main]
async fn main() -> Result<()> {
    let (bevyimage_tx, bevyimage_rx) = tokio::sync::mpsc::channel(1);
    let (movecmd_tx, mut movecmd_rx) = tokio::sync::mpsc::channel::<CommandFromUI>(4);

    let ws_stream = match connect_async("ws://10.8.0.3:8264").await {
        Ok((stream, _)) => stream,
        Err(_) => {
            return Ok(());
        }
    };
    let (mut sender, mut receiver) = ws_stream.split();

    let (encoder_tx, encoder_rx) = broadcast::channel(16);
    let (photo_data_tx, photo_data_rx) = broadcast::channel(32);
    let (image_tx, mut image_rx) = broadcast::channel(1);

    let mut tasks = JoinSet::<Result<()>>::new();
    tasks.spawn(run_decoder(encoder_rx, image_tx));
    tasks.spawn(run_photosaver(photo_data_rx));

    tasks.spawn(async move {
        let mut linear = 0.0;
        let mut angular = 0.0;
        let mut arm = 2400.0;
        loop {
            let movecmd: CommandFromUI = match movecmd_rx.recv().await {
                Some(mc) => mc,
                None => return Ok(()),
            };
            if let Some(drive) = movecmd.drive {
                match drive {
                    Drive::Forward => linear = 0.05,
                    Drive::Stop => linear = 0.0,
                    Drive::Backward => linear = -0.05,
                }
            }
            if let Some(rotate) = movecmd.rotate {
                match rotate {
                    Rotate::Left => angular = 0.1,
                    Rotate::Stop => angular = 0.0,
                    Rotate::Right => angular = -0.1,
                }
            }
            if let Some(a) = movecmd.arm {
                match a {
                    Arm::Up => arm = 2300.0,
                    Arm::Down => arm = 2500.0,
                }
            }
            match movecmd.photo {
                Some(_) => {
                    let pkt = PacketToSlave::TakePhoto;
                    let msg = Message::Binary(pkt.try_to_vec()?);
                    if sender.send(msg).await.is_err() {
                        return Ok(());
                    };
                }
                None => {}
            }
            let velocity_cmd = Velocity { linear, angular };
            let pkt = PacketToSlave::SetVelocity(velocity_cmd);
            let msg = Message::Binary(pkt.try_to_vec()?);
            if sender.send(msg).await.is_err() {
                return Ok(());
            };
            if sender
                .send(Message::Binary(PacketToSlave::SetAngle(arm).try_to_vec()?))
                .await
                .is_err()
            {
                return Ok(());
            };
        }
        // loop {
        //     tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        //     let pkt = PacketToSlave::TakePhoto;
        //     let msg = Message::Binary(pkt.try_to_vec()?);
        //     if sender.send(msg).await.is_err() {
        //         return Ok(());
        //     };
        // }
    });
    tasks.spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Binary(b) => {
                    let cmd = PacketToMaster::try_from_slice(&b)?;
                    match cmd {
                        PacketToMaster::Video(vd) => {
                            let _ = encoder_tx.send(vd);
                        }
                        PacketToMaster::Photo(pd) => {
                            let _ = photo_data_tx.send(pd);
                        }
                        PacketToMaster::Odometry(o) => {
                            info!("got odometry x = {}, y = {}, theta = {}", o.x, o.y, o.theta);
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
        Ok(())
    });
    tasks.spawn(async move {
        loop {
            let img = match image_rx.recv().await {
                Ok(i) => i,
                Err(broadcast::error::RecvError::Lagged(l)) => {
                    error!("lagged for {l} frames");
                    continue;
                }
                Err(_) => return Ok(()),
            };
            let rgba_img = image::DynamicImage::ImageRgb8(img).into_rgba8();
            if bevyimage_tx.send(rgba_img.as_raw().clone()).await.is_err() {
                return Ok(());
            }
        }
    });

    App::new()
        .insert_resource(RemoteControl {
            rx: bevyimage_rx,
            tx: movecmd_tx,
            image_handle: None,
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_systems((move_system, draw_system))
        .run();
    Ok(())
}

#[derive(Resource)]
struct RemoteControl {
    rx: Receiver<Vec<u8>>,
    tx: Sender<CommandFromUI>,
    image_handle: Option<Handle<Image>>,
}

#[derive(Default)]
struct CommandFromUI {
    drive: Option<Drive>,
    rotate: Option<Rotate>,
    arm: Option<Arm>,
    photo: Option<()>,
}

enum Drive {
    Forward,
    Backward,
    Stop,
}

enum Rotate {
    Left,
    Right,
    Stop,
}

enum Arm {
    Up,
    Down,
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>, mut rc: ResMut<RemoteControl>) {
    let size = Extent3d {
        width: VIDEO_WIDTH,
        height: VIDEO_HEIGHT,
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb, //Bgra8UnormSrgb
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // fill image.data with zeroes
    image.resize(size);

    image.data = Vec::from([255; VIDEO_WIDTH as usize * VIDEO_HEIGHT as usize * 4]); // test data

    let image_handle = images.add(image);

    commands.spawn(Camera2dBundle::default());
    commands.spawn(SpriteBundle {
        texture: image_handle.clone(),
        ..default()
    });

    rc.image_handle = Some(image_handle);
}

fn draw_system(mut rc: ResMut<RemoteControl>, mut images: ResMut<Assets<Image>>) {
    match rc.rx.try_recv() {
        Ok(data) => {
            let image_handle = rc.image_handle.as_ref().unwrap();
            let image = images.get_mut(image_handle).unwrap();
            image.data = data;
        }
        Err(TryRecvError::Disconnected) => error!("Image channel is disconected."),
        _ => {}
    }
}

fn move_system(rc: Res<RemoteControl>, key_input: Res<Input<KeyCode>>) {
    let mut move_command = CommandFromUI::default();
    for key in key_input.get_just_pressed() {
        match key {
            KeyCode::W => move_command.drive = Some(Drive::Forward),
            KeyCode::S => move_command.drive = Some(Drive::Backward),
            KeyCode::A => move_command.rotate = Some(Rotate::Left),
            KeyCode::D => move_command.rotate = Some(Rotate::Right),
            KeyCode::Q => move_command.arm = Some(Arm::Up),
            KeyCode::E => move_command.arm = Some(Arm::Down),
            KeyCode::P => move_command.photo = Some(()),
            _ => {}
        }
    }
    for key in key_input.get_just_released() {
        match key {
            KeyCode::W | KeyCode::S => move_command.drive = Some(Drive::Stop),
            KeyCode::A | KeyCode::D => move_command.rotate = Some(Rotate::Stop),
            _ => {}
        }
    }
    if move_command.drive.is_some()
        || move_command.rotate.is_some()
        || move_command.arm.is_some()
        || move_command.photo.is_some()
    {
        if let Err(err) = rc.tx.blocking_send(move_command) {
            warn!("Can't send MoveCommand: {}", err); // TODO: just ignore it?
        }
    }
}
