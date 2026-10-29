pub mod proxmox;

use poise::Command;

use crate::bot::bot::{Data, Error};

pub fn all_commands() -> Vec<Command<Data, Error>> {
  vec![
    proxmox::lxc::lxc(),
  ]
}