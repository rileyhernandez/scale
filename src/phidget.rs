use crate::error::Error;
use crate::scale_trait::*;
use log::info;
use menu::device::Device;
use menu::libra::Config;
use phidget::{Phidget, VoltageRatioInput};
use std::thread::sleep;
use std::time::Duration;

pub struct PhidgetScale {
    vin: VoltageRatioInput,
    device: Device,
    config: Config,
}
#[cfg(feature = "find_phidgets")]
impl PhidgetScale {
    pub const PHIDGET_PRODUCT_ID: u16 = 59;
    pub const PHIDGET_VENDOR_ID: u16 = 1730;
    pub fn get_connected_phidget_ids() -> Result<Vec<i32>, Error> {
        let mut connected_phidgets: Vec<i32> = Vec::with_capacity(4);
        for device in rusb::devices()?.iter() {
            let device_desc = device.device_descriptor()?;
            if device_desc.vendor_id() == Self::PHIDGET_VENDOR_ID
                && device_desc.product_id() == Self::PHIDGET_PRODUCT_ID
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
}
impl Scale for PhidgetScale {
    fn connect(disconnected_scale: DisconnectedScale) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let config = disconnected_scale.get_config();
        let mut vin = VoltageRatioInput::new();
        vin.set_channel(config.load_cell_id)?;
        vin.set_serial_number(config.phidget_id)?;
        vin.open_wait(Duration::from_secs(5))?;
        vin.set_data_interval(config.phidget_sample_period)?;
        info!(
            "Phidget {}, Load Cell {} Connected!",
            vin.serial_number().map_err(Error::Phidget)?,
            vin.channel().map_err(Error::Phidget)?
        );
        sleep(Duration::from_secs(1));
        Ok(Self {
            vin,
            device: disconnected_scale.get_device().clone(),
            config: config.clone(),
        })
    }

    fn disconnect(mut self) -> Result<DisconnectedScale, Error> {
        self.vin.close()?;
        Ok(DisconnectedScale::new(self.device, self.config))
    }

    fn get_device(&self) -> &Device {
        &self.device
    }

    fn get_config(&self) -> &Config {
        &self.config
    }

    fn get_raw_reading(&self) -> Result<f64, Error> {
        self.vin.voltage_ratio().map_err(Error::Phidget)
    }
}
#[cfg(test)]
mod phidget_tests {
    use super::*;

    #[test]
    fn phidget() -> Result<(), Error> {
        let phidget_ids = PhidgetScale::get_connected_phidget_ids()?;
        let phidget_id = phidget_ids.first().ok_or(Error::Initialization)?;
        let config = Config {
            phidget_id: *phidget_id,
            load_cell_id: 0,
            ..Default::default()
        };
        let device = Device::new(menu::device::Model::LibraV0, 0);
        let disconnected_scale = DisconnectedScale::new(device, config);
        let scale = PhidgetScale::connect(disconnected_scale)?;
        _ = scale.get_reading()?;
        scale.disconnect()?;
        Ok(())
    }
}
