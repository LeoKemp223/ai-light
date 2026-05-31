use crate::config::get_lock_path;
use fs2::FileExt;
use std::fs::{self, File, OpenOptions};
use std::io;

pub struct AppLock {
    file: File,
}

impl AppLock {
    pub fn acquire() -> io::Result<Option<Self>> {
        let lock_path = get_lock_path();
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(lock_path)?;

        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self { file })),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(error) => Err(error),
        }
    }
}

impl Drop for AppLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
