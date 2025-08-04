use crate::error::Error;
use log::info;
use menu::device::Device;
use menu::libra::{Config, Libra};
use menu::read::Read;
use phidget::{Phidget, devices::VoltageRatioInput};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

#[cfg(feature = "find_phidgets")]
const PHIDGET_VENDOR_ID: u16 = 1730;
#[cfg(feature = "find_phidgets")]
const PHIDGET_PRODUCT_ID: u16 = 59;

pub struct DisconnectedScale {
    config: Config,
    device: Device,
}
impl DisconnectedScale {
    #[cfg(feature = "find_phidgets")]
    pub fn get_connected_phidget_ids() -> Result<Vec<isize>, Error> {
        let mut connected_phidgets: Vec<isize> = Vec::with_capacity(4);
        for device in rusb::devices()?.iter() {
            let device_desc = device.device_descriptor()?;
            if device_desc.vendor_id() == PHIDGET_VENDOR_ID
                && device_desc.product_id() == PHIDGET_PRODUCT_ID
            {
                let handle = device.open()?;
                if let Some(id) = device_desc.serial_number_string_index() {
                    handle.read_string_descriptor_ascii(id)?;
                    let sn = handle.read_string_descriptor_ascii(id)?;
                    connected_phidgets.push(sn.parse().map_err(|_| Error::ParseInt)?);
                }
            }
        }
        Ok(connected_phidgets)
    }
    pub fn new(config: Config, device: Device) -> Self {
        Self { config, device }
    }
    pub fn from_libra_menu(libra: Libra) -> Self {
        Self::new(libra.config, libra.device)
    }
    pub fn from_config(path: &Path) -> Result<Vec<Self>, Error> {
        Ok(Libra::read_as_vec(path)?
            .into_iter()
            .map(Self::from_libra_menu)
            .collect())
    }
    pub fn connect(self) -> Result<Scale, Error> {
        Scale::new(self.config, self.device)
    }
    pub fn get_device(&self) -> Device {
        self.device.clone()
    }
}
pub struct Scale {
    vin: VoltageRatioInput,
    config: Config,
    device: Device,
    weight_buffer: Vec<f64>,
    last_stable_weight: Option<f64>,
}
impl Scale {
    pub fn new(config: Config, device: Device) -> Result<Self, Error> {
        let mut vin = VoltageRatioInput::new();
        vin.set_channel(config.load_cell_id)
            .map_err(Error::Phidget)?;
        vin.set_serial_number(config.phidget_id)
            .map_err(Error::Phidget)?;
        vin.open_wait(Duration::from_secs(5))
            .map_err(Error::Phidget)?;
        vin.set_data_interval(config.phidget_sample_period)
            .map_err(Error::Phidget)?;
        info!(
            "Phidget {}, Load Cell {} Connected!",
            vin.serial_number().map_err(Error::Phidget)?,
            vin.channel().map_err(Error::Phidget)?
        );
        sleep(Duration::from_secs(1));
        let buffer_length = config.buffer_length;
        Ok(Self {
            vin,
            config,
            device,
            weight_buffer: Vec::with_capacity(buffer_length),
            last_stable_weight: None,
        })
    }
    pub fn restart(&mut self) -> Result<(), Error> {
        self.vin.close().map_err(Error::Phidget)?;
        self.vin
            .open_wait(Duration::from_secs(5))
            .map_err(Error::Phidget)?;
        self.weight_buffer.clear();
        self.last_stable_weight = None;
        sleep(Duration::from_secs(2));
        Ok(())
    }
    pub fn get_device(&self) -> Device {
        self.device.clone()
    }
    pub fn get_raw_reading(&self) -> Result<f64, Error> {
        self.vin.voltage_ratio().map_err(Error::Phidget)
    }
    fn get_reading(&self) -> Result<f64, Error> {
        self.get_raw_reading()
            .map(|r| r * self.config.gain - self.config.offset)
    }
    fn update_buffer(&mut self, weight: f64) {
        if self.weight_buffer.len() < self.config.buffer_length {
            self.weight_buffer.push(weight);
        } else {
            self.weight_buffer.remove(0);
            self.weight_buffer.push(weight);
        }
    }
    fn is_stable(&self) -> bool {
        if self.weight_buffer.len() != self.config.buffer_length {
            return false;
        }
        let max = self
            .weight_buffer
            .iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min = self
            .weight_buffer
            .iter()
            .fold(f64::INFINITY, |a, &b| a.min(b));
        max - min < self.config.max_noise
    }
    pub fn get_weight(&mut self) -> Result<Weight, Error> {
        let reading = self.get_reading()?;
        self.update_buffer(reading);
        if self.is_stable() {
            Ok(Weight::Stable(reading))
        } else {
            Ok(Weight::Unstable(reading))
        }
    }
    pub fn check_for_action(&mut self) -> Option<(Action, f64)> {
        if self.is_stable() {
            let last = self.weight_buffer.last().unwrap();
            if let Some(last_stable) = self.last_stable_weight {
                let delta = last - last_stable;
                if delta.abs() > self.config.max_noise {
                    info!("Scale: {}; Delta: {delta}", self.get_device());
                    self.last_stable_weight = Some(*last);
                    let action = {
                        if delta > 0. {
                            Action::Refilled
                        } else {
                            Action::Served
                        }
                    };
                    return Some((action, delta));
                }
            }
            self.last_stable_weight = Some(*last);
        }
        None
    }
    pub fn get_config(&self) -> Config {
        self.config.clone()
    }
    pub fn disconnect(mut self) -> Result<(), Error> {
        self.vin.close()?;
        Ok(())
    }
    pub fn raw_read_once_settled(&self, stable_samples: usize, timeout: Duration, max_noise_ratio: f64) -> Result<f64, Error> {
        let start_time = std::time::Instant::now();
        let mut stable_count = 0;
        let mut starting_reading = self.get_reading()?;
        while stable_count < stable_samples {
            let curr_reading = self.get_reading()?;
            let max_noise = (max_noise_ratio * starting_reading).abs();
            if (curr_reading - starting_reading).abs() < max_noise {
                stable_count += 1;
            } else {
                stable_count = 0;
                starting_reading = curr_reading;
            }
            sleep(self.config.phidget_sample_period);
            if start_time.elapsed() > timeout {
                return Err(Error::Timeout);
            }
        }
        Ok(starting_reading)
    }
    pub fn weigh_once_settled(
        &self,
        stable_samples: usize,
        timeout: Duration,
        max_noise_ratio: f64,
    ) -> Result<f64, Error> {
        self.raw_read_once_settled(stable_samples, timeout, max_noise_ratio).map(|r| r * self.config.gain - self.config.offset)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use menu::device::Model;
    fn make_scale() -> Result<Scale, Error> {
        let weight_reading =  0.00007940642535686493;
        let empty_reading = -0.000003223307430744171;
        let test_weight = 834.5;

        let config = Config {
            phidget_id: 716588,
            load_cell_id: 0,
            gain: test_weight / (weight_reading - empty_reading),
            offset: test_weight * empty_reading / (weight_reading - empty_reading),
            ..Default::default()
        };
        DisconnectedScale::new(config, Device::new(Model::LibraV0, 0)).connect()
    }
    #[test]
    fn weigh_once_settled() -> Result<(), Error> {
        let scale = make_scale()?;
        let weight = scale.weigh_once_settled(3, Duration::from_secs(10), 0.1)?;
        println!("DEBUG: {weight}");
        Ok(())
    }
}
#[derive(Debug)]
pub enum Weight {
    Stable(f64),
    Unstable(f64),
}
impl Weight {
    pub fn get_amount(&self) -> f64 {
        match self {
            Weight::Stable(value) => *value,
            Weight::Unstable(value) => *value,
        }
    }
}
impl std::fmt::Display for Weight {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Weight::Stable(w) => write!(f, "Stable: {} g", w.trunc() as usize),
            Weight::Unstable(w) => write!(f, "Unstable: {} g", w.trunc() as usize),
        }
    }
}

pub enum Action {
    Served,
    RanOut,
    Refilled,
    Starting,
    Heartbeat,
    Offline,
}
impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Served => write!(f, "Served"),
            Action::RanOut => write!(f, "RanOut"),
            Action::Refilled => write!(f, "Refilled"),
            Action::Starting => write!(f, "Starting"),
            Action::Heartbeat => write!(f, "Heartbeat"),
            Action::Offline => write!(f, "Offline"),
        }
    }
}
