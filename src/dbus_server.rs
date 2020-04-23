use crate::session::SessionManager;
use std::sync::{Arc, Mutex};


pub fn run_dbus_server_new_thread(sesman: Arc<Mutex<SessionManager>>) {
    std::thread::spawn(move ||run_dbus_server(sesman));
}

pub fn run_dbus_server(sesman: Arc<Mutex<SessionManager>>) {
    let mut con = rustbus::RpcConn::new(
        rustbus::Conn::connect_to_bus(rustbus::get_system_bus_path().unwrap(), false).unwrap(),
    );

    con.send_message(&mut rustbus::standard_messages::hello(), None)
        .unwrap();
    con.send_message(
        &mut rustbus::standard_messages::request_name(
            "org.freedesktop.login2".into(),
            rustbus::standard_messages::DBUS_REQUEST_NAME_REPLY_PRIMARY_OWNER,
        ),
        None,
    )
    .unwrap();

    loop {
        let call = con.wait_call(None).unwrap();
        if call.object.eq(&Some("/org/freedesktop/login1".to_owned()))
            && call
                .interface
                .eq(&Some("org.freedesktop.login1.Manager".to_owned()))
        {
            match call.member.unwrap().as_str() {
                "CreateSession" => {
                    let uid = call.params[0].as_u32().unwrap();
                    let pid = call.params[1].as_u32().unwrap();
                    let tty = call.params[3].as_str().unwrap();
                    let seatid = call.params[3].as_str().unwrap();

                    let sesman_locked: &mut SessionManager = &mut *sesman.lock().unwrap();
                    sesman_locked.add_new_session(*pid as i32, *uid as i32, seatid, Some(tty.to_owned())).unwrap();
                }
                "ReleaseSession" => {
                    let session_id = call.params[0].as_str().unwrap();
                    let sesman_locked: &mut SessionManager = &mut *sesman.lock().unwrap();
                    sesman_locked.remove_session(&session_id.into());
                }
                "ActivateSessionOnSeat" => {
                    let session_id = call.params[0].as_str().unwrap();
                    let seat_id = call.params[1].as_str().unwrap();
                    let sesman_locked: &mut SessionManager = &mut *sesman.lock().unwrap();
                    sesman_locked.session_aquire_seat(&session_id.into(), &seat_id.into()).unwrap();
                }
                other => {
                    println!("Call to unsupported member {}", other);
                    //ignore
                }
            }
        }
    }
}
