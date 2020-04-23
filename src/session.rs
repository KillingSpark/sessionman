use crate::cgroupv2;
use crate::seat;
use acl_rs as acl;
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct SessionId {
    id: String,
}

impl From<&str> for SessionId {
    fn from(r: &str) -> SessionId {
        SessionId { id: r.into() }
    }
}

#[derive(Debug)]
pub struct SessionManager {
    sessions: HashMap<SessionId, Session>,
    seats: HashMap<seat::SeatId, seat::Seat>,
    session_to_seat: HashMap<SessionId, seat::SeatId>,
    seat_to_session: HashMap<seat::SeatId, SessionId>,
    session_counter: u64,
    cgroup: cgroupv2::Cgroup,
}

#[derive(Debug)]
pub struct Session {
    id: SessionId,
    name: String,
    uid: i32,
    tty: Option<String>,
    cgroup: cgroupv2::Cgroup,
}

#[derive(Debug)]
pub enum Error {
    SessionExistsOnTTY(String),
    CantCreateCgroup(cgroupv2::Error),
    CantMoveCgroup(cgroupv2::Error),
}

#[derive(Debug)]
pub enum AquireError {
    SeatTaken(SessionId),
    UnknownSeat(seat::SeatId),
    UnknownSession(SessionId),
}

impl SessionManager {
    pub fn new() -> Result<SessionManager, cgroupv2::Error> {
        let mut top_cgroup = cgroupv2::Cgroup::new_self()?;
        top_cgroup.make_inner_node("sessionman_self")?;
        Ok(SessionManager {
            sessions: HashMap::new(),
            seats: HashMap::new(),
            session_to_seat: HashMap::new(),
            seat_to_session: HashMap::new(),
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

    pub fn get_session_for_tty(&self, tty: &String) -> Option<SessionId> {
        for ses in self.sessions.values() {
            if let Some(ses_tty) = &ses.tty {
                if ses_tty.eq(tty) {
                    return Some(ses.id.clone());
                }
            }
        }
        None
    }

    pub fn tty_changed(&mut self, new_tty: &String) {
        if let Some(session_id) = self.get_session_for_tty(new_tty) {
            self.session_aquire_seat(&session_id, &"seat0".into());
        }
    }

    pub fn remove_session(&mut self, session_id: &SessionId) {
        if let Some(seat_id) = self.session_to_seat.remove(session_id) {
            self.session_leave_seat(session_id, &seat_id);
        }
        self.sessions.remove(session_id);
    }

    pub fn session_leave_seat(
        &mut self,
        session_id: &SessionId,
        seat_id: &seat::SeatId,
    ) -> Result<(), AquireError> {
        let seat = if let Some(seat) = self.seats.get(seat_id) {
            seat
        } else {
            return Err(AquireError::UnknownSeat(seat_id.clone()));
        };
        let session = if let Some(session) = self.sessions.get(session_id) {
            session
        } else {
            return Err(AquireError::UnknownSession(session_id.clone()));
        };

        // Remove RW access for this UID to the devices
        let session_uid = nix::unistd::Uid::from_raw(session.uid as u32);
        for dev in seat.devices.values() {
            let mut devacl = acl::Acl::for_file(&dev.dev_name, &acl::AclType::TypeAccess).unwrap();
            let mut entry = devacl.get_entry(&acl::EntryId::FirstEntry);
            loop {
                match entry {
                    Err(_) => break, // TODO handle error!
                    Ok(None) => break,
                    Ok(Some(entry)) => match entry.get_qualifier().unwrap() {
                        acl::Qualifier::User(uid) => {
                            if uid == session_uid {
                                // Found the entry related to this sessions user. Delete it.
                                devacl.delete_entry(entry).unwrap();
                            }
                        }
                        acl::Qualifier::Group(_gid) => {
                            // we do not alter groups here
                        }
                    },
                }
                entry = devacl.get_entry(&acl::EntryId::NextEntry);
            }

            devacl
                .set_for_file(&dev.dev_name, &acl::AclType::TypeAccess)
                .unwrap();
        }
        Ok(())
    }

    pub fn session_aquire_seat(
        &mut self,
        session_id: &SessionId,
        seat_id: &seat::SeatId,
    ) -> Result<(), AquireError> {
        if self.seat_to_session.contains_key(seat_id) {
            return Err(AquireError::SeatTaken(session_id.clone()));
        }
        // first push old session out of the seat
        if let Some(old_session) = self.seat_to_session.get(seat_id) {
            // TODO handle errors
            let old_session = old_session.clone();
            self.session_leave_seat(&old_session, seat_id)?;
        }

        // Then allow the new session access to the devs
        let seat = if let Some(seat) = self.seats.get(seat_id) {
            seat
        } else {
            return Err(AquireError::UnknownSeat(seat_id.clone()));
        };
        let session = if let Some(session) = self.sessions.get(session_id) {
            session
        } else {
            return Err(AquireError::UnknownSession(session_id.clone()));
        };

        let uid = session.uid as u32;
        for dev in seat.devices.values() {
            let mut devacl = acl::Acl::for_file(&dev.dev_name, &acl::AclType::TypeAccess).unwrap();
            let mut new_entry = devacl.create_entry().unwrap();
            let tag = acl::AclTag::User;
            new_entry.set_tag_type(&tag).unwrap();
            new_entry
                .set_qualifier(&acl::Qualifier::User(nix::unistd::Uid::from_raw(uid)))
                .unwrap();
            let mut permset = new_entry.get_permset().unwrap();
            permset.add_perm(acl::AclPerm::Read).unwrap();
            permset.add_perm(acl::AclPerm::Write).unwrap();

            devacl
                .set_for_file(&dev.dev_name, &acl::AclType::TypeAccess)
                .unwrap();
        }

        Ok(())
    }

    pub fn add_new_session(
        &mut self,
        pid: i32,
        uid: i32,
        seat: &str,
        tty: Option<String>,
    ) -> Result<SessionId, Error> {
        if tty.is_some() {
            let mut tty_has_session = false;
            for ses in self.sessions.values() {
                if ses.tty == tty {
                    tty_has_session = true;
                    break;
                }
            }
            if tty_has_session {
                return Err(Error::SessionExistsOnTTY(tty.unwrap()));
            }
        }

        let name = format!("User_{}_Session_{}", uid, self.session_counter.to_string());
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
        self.session_to_seat.insert(session.id.clone(), seat.into());
        self.sessions.insert(session.id.clone(), session);
        Ok(id.clone())
    }
}
