use crate::error::Error;
use log::{info};
use menu::device::Device;
use menu::libra::{Config, Libra};
use menu::read::Read;
use phidget::{Phidget, devices::VoltageRatioInput};
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

const BUFFER_LENGTH: usize = 20;
const MAX_NOISE: f64 = 3.0;

pub struct DisconnectedScale {
    config: Config,
    device: Device,
}
impl DisconnectedScale {
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
        Scale::new(self.config, self.device, Duration::from_millis(100))
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
    pub fn new(config: Config, device: Device, sample_period: Duration) -> Result<Self, Error> {
        let mut vin = VoltageRatioInput::new();
        vin.set_channel(config.load_cell_id)
            .map_err(Error::Phidget)?;
        vin.set_serial_number(config.phidget_id)
            .map_err(Error::Phidget)?;
        vin.open_wait(Duration::from_secs(5))
            .map_err(Error::Phidget)?;
        vin.set_data_interval(sample_period)
            .map_err(Error::Phidget)?;
        info!(
            "Phidget {}, Load Cell {} Connected!",
            vin.serial_number().map_err(Error::Phidget)?,
            vin.channel().map_err(Error::Phidget)?
        );
        sleep(Duration::from_secs(1));
        Ok(Self {
            vin,
            config,
            device,
            weight_buffer: Vec::with_capacity(BUFFER_LENGTH),
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
        if self.weight_buffer.len() < BUFFER_LENGTH {
            self.weight_buffer.push(weight);
        } else {
            self.weight_buffer.remove(0);
            self.weight_buffer.push(weight);
        }
    }
    fn is_stable(&self) -> bool {
        if self.weight_buffer.len() != BUFFER_LENGTH {
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
        max - min < MAX_NOISE
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
                if delta.abs() > MAX_NOISE {
                    info!("Scale: {}; Delta: {delta}", self.get_device());
                    self.last_stable_weight = Some(*last);
                    let action = {
                        if delta > 0. {
                            Action::Refilled
                        } else {
                            Action::Served
                        }
                    };
                    return Some((action, delta))
                }
            }
            self.last_stable_weight = Some(*last);
        }
        None
    }
    pub fn get_config(&self) -> Config {
        self.config.clone()
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
}
impl std::fmt::Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::Served => write!(f, "Served"),
            Action::RanOut => write!(f, "Ran Out"),
            Action::Refilled => write!(f, "Refilled"),
            Action::Starting => write!(f, "Starting"),
        }
    }
}