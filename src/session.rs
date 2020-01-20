use crate::cgroupv2;
use crate::seat;
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct SessionId {
    id: String,
}

#[derive(Debug)]
pub struct SessionManager {
    sessions: HashMap<SessionId, Session>,
    seats: HashMap<String, seat::Seat>,
    session_to_seat: HashMap<String, String>,
    session_counter: u64,
    cgroup: cgroupv2::Cgroup,
}

#[derive(Debug)]
pub struct Session {
    id: SessionId,
    name: String,
    uid: i32,
    tty: String,
    cgroup: cgroupv2::Cgroup,
}

#[derive(Debug)]
pub enum Error {
    SessionExistsOnTTY(String),
    CantCreateCgroup(cgroupv2::Error),
    CantMoveCgroup(cgroupv2::Error),
}

impl SessionManager {
    pub fn new() -> Result<SessionManager, cgroupv2::Error> {
        let mut top_cgroup = cgroupv2::Cgroup::new_self()?;
        top_cgroup.make_inner_node("sessionman_self")?;
        Ok(SessionManager {
            sessions: HashMap::new(),
            seats: HashMap::new(),
            session_to_seat: HashMap::new(),
            session_counter: 0,
            cgroup: top_cgroup,
        })
    }

    pub fn reload_seats(&mut self) -> Result<(), seat::Error> {
        if self.seats.is_empty() {
            self.seats
                .insert("seat0".into(), seat::Seat::get_seat("seat0")?);
        } else {
            for seat in self.seats.values_mut() {
                seat.reload_devices()?;
            }
        }
        Ok(())
    }

    pub fn add_new_session(
        &mut self,
        pid: i32,
        uid: i32,
        seat: &str,
        tty: String,
    ) -> Result<SessionId, Error> {
        let mut tty_has_session = false;
        for ses in self.sessions.values() {
            if ses.tty == tty {
                tty_has_session = true;
                break;
            }
        }
        if tty_has_session {
            return Err(Error::SessionExistsOnTTY(tty));
        }

        let name = format!("Session-{}", self.session_counter.to_string());
        let mut cgroup = self
            .cgroup
            .new_leaf(&name)
            .map_err(|e| Error::CantCreateCgroup(e))?;
        cgroup
            .move_into(pid)
            .map_err(|e| Error::CantMoveCgroup(e))?;

        let id = SessionId {
            id: self.session_counter.to_string(),
        };

        let session = Session {
            name,
            id: id.clone(),
            uid: uid,
            tty: tty,
            cgroup,
        };
        self.session_counter += 1;
        self.session_to_seat
            .insert(session.name.clone(), seat.into());
        self.sessions.insert(session.id.clone(), session);
        Ok(id.clone())
    }
}
