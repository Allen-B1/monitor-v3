use std::{fmt, hash::Hash};
use std::collections::HashMap;
use serde::{Deserialize,Serialize};

use crate::{ActiveProgram, Program};

pub type DeviceID = u16;

#[derive(Clone, Serialize, Deserialize)]
pub struct Add {
    pub device: DeviceID,
    pub active: HashMap<ActiveProgram, u32>,
    pub open: HashMap<Program, u32>,
}

impl Add {
    pub fn new(device: DeviceID) -> Self {
        Add { device, active: HashMap::new(), open: HashMap::new() }
    }
}


#[derive(Clone, Default, Serialize, Deserialize)]
pub struct DeviceData {
    #[serde(rename = "type")]
    pub type_: DeviceType,
    pub os: String,
    pub distro: Option<String>,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Desktop,
    Laptop,
    Phone,
    Tablet,
    Other
}

impl Default for DeviceType {
    fn default() -> Self {
        DeviceType::Other
    }
}

impl fmt::Display for DeviceData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.os)?;
        if let Some(distro) = &self.distro {
            write!(f, ": {}", distro)?;
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: DeviceID,
    pub data: DeviceData,
}