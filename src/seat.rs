use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct SeatId {
    id: String,
}

#[derive(Debug)]
pub struct Seat {
    pub id: SeatId,
    pub devices: HashMap<String, Device>,
}

impl From<&str> for SeatId {
    fn from(r: &str) -> SeatId {
        SeatId { id: r.to_owned() }
    }
}

#[derive(Debug)]
pub struct Device {
    pub dev_name: PathBuf,
    pub seat_tag: Option<String>,
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
                dev_name: PathBuf::from("/dev/some/devfile"),
                seat_tag: Some("seat0".into()),
            },
        );
        Ok(Seat {
            devices,
            id: seat_tag.into(),
        })
    }
    pub fn reload_devices(&mut self) -> Result<(), Error> {
        self.devices.insert(
            "Monitor".into(),
            Device {
                dev_name: PathBuf::from("/dev/some/devfile"),
                seat_tag: Some("seat0".into()),
            },
        );
        Ok(())
    }
}
