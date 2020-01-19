use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Seat {
    devices: HashMap<String, Device>,
}

#[derive(Debug)]
pub struct Device {
    pub dev_path: PathBuf,
}

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
}

impl Seat {
    pub fn get_seat(seat_tag: &str) -> Result<Seat, Error> {
        // find devices in udev list
        let mut devices = HashMap::new();
        devices.insert(
            "Monitor".into(),
            Device {
                dev_path: PathBuf::from("/sys/device/some/device/path"),
            },
        );

        Ok(Seat { devices })
    }

    pub fn reload_devices(&mut self) -> Result<(), Error> {
        self.devices.insert(
            "Monitor".into(),
            Device {
                dev_path: PathBuf::from("/sys/device/some/device/path"),
            },
        );
        Ok(())
    }
}
