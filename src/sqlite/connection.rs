extern crate libsqlite3_sys as ffi;

use std::ffi::CString;
use std::ptr::{self, NonNull};
use std::path::Path;

pub struct Connection {
    pub raw: NonNull<ffi::sqlite3>,
}

impl Connection {
    pub fn open<T: AsRef<Path>>(path: T) -> Result<Self, String> {
        let path = match path.as_ref().to_str() {
            Some(path) => {
                match CString::new(path) {
                    Ok(string) => string,
                    _ => return Err(format!("invalid path: {}", path)),
                }
            },
            _ => return Err(format!("failed to open path: {:?}", path.as_ref())),
        };
        let mut conn_ptr = ptr::null_mut();

        let status = unsafe { ffi::sqlite3_open_v2(
            path.as_ptr(),
            &mut conn_ptr,
            ffi::SQLITE_OPEN_CREATE | ffi::SQLITE_OPEN_READWRITE,
            ptr::null())
        };

        match status {
            ffi::SQLITE_OK =>
                Ok(Connection { raw: unsafe { NonNull::new_unchecked(conn_ptr)}} ),
            _ =>
                Err("failed to connect".to_string()),
        }
    }
}
