//! Output device discovery. Devices are identified by the platform id, which survives
//! renames and reboots, unlike the human-readable name.

use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::cpal::{self, Device};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn list() -> Vec<OutputDevice> {
    let host = cpal::default_host();
    let default_id = host.default_output_device().as_ref().and_then(device_id);
    let Ok(devices) = host.output_devices() else { return Vec::new() };

    let mut list: Vec<OutputDevice> = devices
        .filter_map(|device| {
            let id = device_id(&device)?;
            let name = device.description().ok()?.name().to_owned();
            Some(OutputDevice {
                is_default: default_id.as_deref() == Some(id.as_str()),
                id,
                name,
            })
        })
        .collect();
    list.sort_by_key(|device| device.name.to_lowercase());
    list
}

pub fn find(id: &str) -> Option<Device> {
    cpal::default_host()
        .output_devices()
        .ok()?
        .find(|device| device_id(device).as_deref() == Some(id))
}

pub fn default_id() -> Option<String> {
    cpal::default_host().default_output_device().as_ref().and_then(device_id)
}

pub fn device_id(device: &Device) -> Option<String> {
    device.id().ok().map(|id| id.1)
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "lists the machine's audio outputs"]
    fn lists_outputs() {
        for device in super::list() {
            println!("{} | {} | default={}", device.name, device.id, device.is_default);
        }
        println!("default id: {:?}", super::default_id());
    }
}
