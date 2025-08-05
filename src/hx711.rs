use crate::error::Error;
use crate::scale_trait::*;
use log::info;
use menu::device::Device;
use menu::libra::Config;
use phidget::{Phidget, VoltageRatioInput};
use std::thread::sleep;
use std::time::Duration;

pub struct Hx711Scale {
    device: Device,
    config: Config,
}


// #[cfg(test)]
// mod phidget_tests {
//     use super::*;
//
//     #[test]
//     fn phidget() -> Result<(), Error> {
//         let phidget_ids = Hx711Scale::get_connected_phidget_ids()?;
//         let phidget_id = phidget_ids.first().ok_or(Error::Initialization)?;
//         let config = Config {
//             phidget_id: *phidget_id,
//             load_cell_id: 0,
//             ..Default::default()
//         };
//         let device = Device::new(menu::device::Model::LibraV0, 0);
//         let disconnected_scale = DisconnectedScale::new(device, config);
//         let scale = Hx711Scale::connect(disconnected_scale)?;
//         _ = scale.get_reading()?;
//         scale.disconnect()?;
//         Ok(())
//     }
// }
