use std::fs;
use std::io::Write;
use std::path::PathBuf;

const PROCESS_FILE_NAME: &'static str = "cgroup.procs";
const EVENTS_FILE_NAME: &'static str = "cgroup.events";
const FREEZE_FILE_NAME: &'static str = "cgroup.freeze";
const CGROUPSV2_MOUNT_PATH: &'static str = "/sys/fs/cgroup/unified";

pub struct Cgroup {
    self_path: PathBuf,
}

impl std::fmt::Debug for Cgroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Path: {:?}, populated: {:?}, frozen: {:?}",
            self.self_path,
            self.is_populated(),
            self.is_frozen()
        )
    }
}

#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    NoCgroupv2,
    CantCreate(std::io::Error),
    InvalidPidEntry(String),
}

fn find_own_cgroup() -> Result<String, Error> {
    let proc_content = fs::read_to_string("/proc/self/cgroup").map_err(|e| Error::IoError(e))?;
    let lines = proc_content.split('\n');
    for line in lines {
        if line.starts_with("0::") {
            return Ok(line[3..].to_owned());
        }
    }
    Err(Error::NoCgroupv2)
}

impl Cgroup {
    pub fn new_self() -> Result<Cgroup, Error> {
        let self_path = find_own_cgroup()?;
        let full_self_path = PathBuf::from(CGROUPSV2_MOUNT_PATH).join(&self_path[1..]);
        if !full_self_path.exists() {
            fs::create_dir_all(&full_self_path).map_err(|e| Error::CantCreate(e))?;
        }
        Ok(Cgroup {
            self_path: full_self_path,
        })
    }

    pub fn new_sub_group(&mut self, new_group_name: &str) -> Result<Cgroup, Error> {
        let full_new_path = self.self_path.join(new_group_name);
        if !full_new_path.exists() {
            fs::create_dir_all(&full_new_path).map_err(|e| Error::CantCreate(e))?;
        }
        Ok(Cgroup {
            self_path: full_new_path,
        })
    }

    pub fn is_frozen(&self) -> Result<bool, Error> {
        let content = fs::read_to_string(self.self_path.join(EVENTS_FILE_NAME))
            .map_err(|e| Error::IoError(e))?;
        Ok(content.contains("frozen 1"))
    }

    pub fn is_populated(&self) -> Result<bool, Error> {
        let content = fs::read_to_string(self.self_path.join(EVENTS_FILE_NAME))
            .map_err(|e| Error::IoError(e))?;
        Ok(content.contains("populated 1"))
    }

    pub fn move_into(&mut self, pid: i32) -> Result<(), Error> {
        let mut procs_file = fs::OpenOptions::new()
            .write(true)
            .open(self.self_path.join(PROCESS_FILE_NAME))
            .map_err(|e| Error::IoError(e))?;

        procs_file
            .write_all(pid.to_string().as_bytes())
            .map_err(|e| Error::IoError(e))?;

        Ok(())
    }

    pub fn freeze(&mut self) -> Result<(), Error> {
        let mut freeze_file = fs::OpenOptions::new()
            .write(true)
            .open(self.self_path.join(FREEZE_FILE_NAME))
            .map_err(|e| Error::IoError(e))?;

        freeze_file
            .write_all(&[b'1'])
            .map_err(|e| Error::IoError(e))?;

        Ok(())
    }

    pub fn thaw(&mut self) -> Result<(), Error> {
        let mut freeze_file = fs::OpenOptions::new()
            .write(true)
            .open(self.self_path.join(FREEZE_FILE_NAME))
            .map_err(|e| Error::IoError(e))?;

        freeze_file
            .write_all(&[b'0'])
            .map_err(|e| Error::IoError(e))?;

        Ok(())
    }

    pub fn get_pids(&mut self) -> Result<Vec<i32>, Error> {
        let content = fs::read_to_string(self.self_path.join(PROCESS_FILE_NAME))
            .map_err(|e| Error::IoError(e))?;

        let mut pids = Vec::new();
        for line in content.split('\n') {
            if !line.is_empty() {
                if let Ok(pid) = line.parse::<i32>() {
                    pids.push(pid);
                } else {
                    return Err(Error::InvalidPidEntry(line.to_owned()));
                }
            }
        }

        Ok(pids)
    }
}
