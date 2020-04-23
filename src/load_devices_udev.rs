pub fn load_devices_from_udev() -> Vec<crate::seat::Device> {
    let mut devs = Vec::new();

    let raw_data = std::process::Command::new("udevadm")
        .arg("info")
        .arg("-e")
        .output()
        .unwrap()
        .stdout;

    let mut start = 0;
    for idx in 1..raw_data.len() {
        if raw_data[idx - 1] == b'\n' && raw_data[idx] == b'\n' {
            if let Some(dev) = parse_dev(&raw_data[start..idx - 1]) {
                devs.push(dev);
            }
            start = idx + 1;
        }
    }

    devs
}

pub fn parse_dev(raw: &[u8]) -> Option<crate::seat::Device> {
    // FIXME this only reports back a subset of all devices that should be returned
    // Apparently not all devices are marked with the tag :seat: Sometimes just the parent device is.
    //
    // Maybe it is necessary to parse all devs in detail and lookup if any parents are marked
    let lines = raw.split(|e| *e == b'\n');
    let mut dev_name = None;
    let seat_tag = None;
    let mut seat_assignable = false;

    for line in lines {
        let line = String::from_utf8(line.to_vec()).unwrap();
        println!("Line: {}", line);
        if line.starts_with("E: DEVNAME=") {
            dev_name = Some(line.trim_start_matches("E: DEVNAME=").into());
        }
        if line.starts_with("E: TAGS=") && line.contains(":seat:") {
            seat_assignable = true;
        }
    }

    if !seat_assignable {
        return None;
    }

    if let Some(dev_name) = dev_name {
        Some(crate::seat::Device {
            dev_name: dev_name,
            seat_tag,
        })
    } else {
        None
    }
}
