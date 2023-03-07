use std::time::Duration;

use bevy::{
    prelude::*,
    render::{
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
    },
};
use crossbeam::channel::{Receiver, Sender, TryRecvError};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_systems((move_system, draw_system))
        .run();
}

#[derive(Resource)]
struct RemoteControl {
    rx: Receiver<Vec<u8>>,
    tx: Sender<MoveCommand>,
    image_handle: Handle<Image>,
}

#[derive(Default)]
struct MoveCommand {
    drive: Option<Drive>,
    rotate: Option<Rotate>,
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

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let size = Extent3d {
        width: 640,
        height: 480,
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

    image.data = Vec::from([255; 640 * 480 * 4]); // test data

    let image_handle = images.add(image);

    commands.spawn(Camera2dBundle::default());
    commands.spawn(SpriteBundle {
        texture: image_handle.clone(),
        ..default()
    });

    let (_tx1, rx1) = crossbeam::channel::bounded(1);
    let (tx2, _rx2) = crossbeam::channel::bounded(1);

    commands.insert_resource(RemoteControl {
        rx: rx1,
        tx: tx2,
        image_handle
    });
}

fn draw_system(rc: Res<RemoteControl>, mut images: ResMut<Assets<Image>>) {
    match rc.rx.try_recv() {
        Ok(data) => {
            let image = images.get_mut(&rc.image_handle).unwrap();
            image.data = data;
        },
        Err(TryRecvError::Disconnected) => error!("Image channel is disconected."),
        _ => {}
    }
}


fn move_system(rc: Res<RemoteControl>, key_input: Res<Input<KeyCode>>) {
    let mut move_command = MoveCommand::default();
    for key in key_input.get_just_pressed() {
        match key {
            KeyCode::W => move_command.drive = Some(Drive::Forward),
            KeyCode::S => move_command.drive = Some(Drive::Backward),
            KeyCode::A => move_command.rotate = Some(Rotate::Left),
            KeyCode::D => move_command.rotate = Some(Rotate::Right),
            _ => {},
        }
    }
    for key in key_input.get_just_released() {
        match key {
            KeyCode::W | KeyCode::S => move_command.drive = Some(Drive::Stop),
            KeyCode::A | KeyCode::D => move_command.rotate = Some(Rotate::Stop),
            _ => {},
        }
    }
    if move_command.drive.is_some() || move_command.rotate.is_some() {
        if let Err(err) = rc.tx.send_timeout(move_command, Duration::from_millis(100)) {
            warn!("Can't send MoveCommand: {}", err); // TODO: just ignore it?
        }
    }
}
