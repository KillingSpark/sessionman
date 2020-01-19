mod cgroupv2;
mod seat;
mod session;

fn main() {
    let mut child = std::process::Command::new("/bin/sleep").arg("100").spawn().unwrap();
    let mut sesman = session::SessionManager::new().unwrap();
    sesman.reload_seats().unwrap();
    let _id = sesman.add_new_session(child.id() as i32, 1000, "seat0", "tty3".into()).unwrap();

    println!("sesman: {:?}", sesman);

    child.kill().unwrap();
}
