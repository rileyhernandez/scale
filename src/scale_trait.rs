use crate::error::Error;
use menu::device::Device;
use menu::libra::{Config, Libra};
use menu::read::Read;
use std::path::Path;

// pub trait DisconnectedScale {
//     fn new(device: Device, config: Config) -> Self
//     where
//         Self: Sized;
//     fn from_libra_menu(libra: Libra) -> Self
//     where
//         Self: Sized,
//     {
//         Self::new(libra.device, libra.config)
//     }
//     fn from_config_file(path: &Path) -> Result<Vec<Self>, Error>
//     where
//         Self: Sized,
//     {
//         Ok(Libra::read_as_vec(path)?
//             .into_iter()
//             .map(Self::from_libra_menu)
//             .collect())
//     }
//     fn connect(self) -> Result<Box<dyn ConnectedScale>, Error>;
// }

pub struct DisconnectedScale {
    device: Device,
    config: Config,
}
impl DisconnectedScale {
    pub fn new(device: Device, config: Config) -> Self {
        Self { device, config }
    }
    pub fn from_libra_menu(libra: Libra) -> Self {
        Self::new(libra.device, libra.config)
    }
    pub fn from_config_file(path: &Path) -> Result<Vec<Self>, Error> {
        Ok(Libra::read_as_vec(path)?
            .into_iter()
            .map(Self::from_libra_menu)
            .collect())
    }
    pub fn get_device(&self) -> &Device {
        &self.device
    }
    pub fn get_config(&self) -> &Config {
        &self.config
    }
}

pub trait Scale {
    fn connect(disconnected_scale: DisconnectedScale) -> Result<Self, Error>
    where
        Self: Sized;
    fn disconnect(self) -> Result<DisconnectedScale, Error>;
    fn get_device(&self) -> &Device;
    fn get_config(&self) -> &Config;
    fn get_gain(&self) -> &f64 {
        &self.get_config().gain
    }
    fn get_offset(&self) -> &f64 {
        &self.get_config().offset
    }
    fn get_raw_reading(&self) -> Result<f64, Error>;
    fn get_reading(&self) -> Result<f64, Error> {
        Ok(self.get_gain() * self.get_raw_reading()? - self.get_offset())
    }
}
