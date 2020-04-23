mod cgroupv2;
mod dbus_server;
mod load_devices_udev;
mod seat;
mod session;

use std::sync::{Arc, Mutex};

fn main() {
    for dev in load_devices_udev::load_devices_from_udev() {
        println!("Device found: {:?}", dev);
    }
    return;

    let mut sesman = session::SessionManager::new().unwrap();
    let mut child = std::process::Command::new("/bin/sleep")
        .arg("100")
        .spawn()
        .unwrap();
    sesman.reload_seats().unwrap();
    let session_id = sesman
        .add_new_session(child.id() as i32, 1000, "seat0", Some("tty3".into()))
        .unwrap();
    println!("sesman: {:?}", sesman);
    //let seat_id = "seat0".into();
    //sesman.session_aquire_seat(&session_id, &seat_id).unwrap();
    //sesman.session_leave_seat(&session_id, &seat_id).unwrap();
    child.kill().unwrap();

    let inotify = nix::sys::inotify::Inotify::init(nix::sys::inotify::InitFlags::empty()).unwrap();
    let _watch = inotify
        .add_watch(
            "/sys/class/tty/tty0/active",
            nix::sys::inotify::AddWatchFlags::IN_MODIFY,
        )
        .unwrap();

    let sesman = Arc::new(Mutex::new(sesman));

    dbus_server::run_dbus_server_new_thread(Arc::clone(&sesman));

    let mut current_tty = std::fs::read_to_string("/sys/class/tty/tty0/active").unwrap();
    loop {
        for ev in inotify.read_events() {
            let new_tty = std::fs::read_to_string("/sys/class/tty/tty0/active").unwrap();
            if new_tty != current_tty {
                current_tty = new_tty;
                println!("Session change!: {:?}", ev);
                println!("New tty active: {}", current_tty);
                sesman.lock().unwrap().tty_changed(&current_tty);
            }
        }
    }
}
