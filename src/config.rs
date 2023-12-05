use std::{fs::OpenOptions, io::Read, path::PathBuf};

use serde::Deserialize;

use crate::controller::CoOrdinates;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum GraphType {
    Linear,
    Step,
}

#[derive(Deserialize, Debug, Clone)]
pub struct AwcConfig {
    pub disable_power_mode_on_startup: bool,
    pub cpu: DeviceInfo,
    pub gpu: DeviceInfo,
    pub interval: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DeviceInfo {
    pub graph_type: GraphType,
    pub graph: Vec<CoOrdinates>,
    pub sensor: u8,
    pub fan: u8,
}

impl AwcConfig {
    pub fn from_file_path(file_path: &str) -> Self {
        let mut s = String::with_capacity(1024);
        OpenOptions::new()
            .read(true)
            .open(file_path)
            .unwrap()
            .read_to_string(&mut s);
        json5::from_str(&s).unwrap()
    }

    pub fn new() -> Self {
        if let Some(dir) = dirs::config_dir() {
            print!("Config Dir {}\n", dir.to_str().unwrap());
            let config_file_path = "/etc/awc.conf";
            if PathBuf::from(config_file_path).exists() {
                print!("Using config file from {config_file_path}\n");
                return Self::from_file_path(&config_file_path);
            }
        }

        panic!("Can't find Config file")
    }
}
