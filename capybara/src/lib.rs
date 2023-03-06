pub mod camera;
pub mod decoder;
pub mod encoder;
pub mod muskrat;
pub mod photosaver;
pub mod phototaker;
pub mod radio;
pub mod ros;
pub mod servo;
pub mod ws;
pub mod yuvrgb;

use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use log::*;
use tokio::task::JoinSet;

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Odometry {
    pub x: f64,
    pub y: f64,
    pub theta: f64,
}

impl Odometry {
    pub fn distance_to(&self, other: &Self) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Velocity {
    pub linear: f64,
    pub angular: f64,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub enum PacketToSlave {
    TakePhoto,
    SetVelocity(Velocity),
    SetAngle(f64),
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug)]
pub enum PacketToMaster {
    Video(Vec<u8>),
    Photo(Vec<u8>),
    Odometry(Odometry),
}

pub async fn wait_tasks(mut tasks: JoinSet<Result<()>>) {
    while let Some(res) = tasks.join_next().await {
        match res {
            Ok(_) => error!("task exited"),
            Err(e) => error!("task failed: {e}"),
        }
    }
}
