use std::{ffi::CStr, fs::File, os::fd::FromRawFd};

#[allow(non_camel_case_types)]
type c_int = i32;
#[allow(non_camel_case_types)]
type c_char = i8;

// Declare external C functions from <fcntl.h> and <stdlib.h>
unsafe extern "C" {
    fn posix_openpt(flags: c_int) -> c_int;
    fn grantpt(fd: c_int) -> c_int;
    fn unlockpt(fd: c_int) -> c_int;
    fn ptsname(fd: c_int) -> *mut c_char;
}

// Needed constants (from fcntl.h)
const O_RDWR: i32 = 0o00000002;
const O_NOCTTY: i32 = 0o00000400;

pub fn get_pts_pair() -> std::io::Result<(File, &'static str)> {
    use std::io::Error;
    unsafe {
        let master_fd: c_int = posix_openpt(O_RDWR | O_NOCTTY);
        if master_fd < 0 {
            panic!("posix_openpt failed: {}", Error::last_os_error());
        }

        if grantpt(master_fd) != 0 {
            panic!("grantpt failed: {}", Error::last_os_error());
        }
        if unlockpt(master_fd) != 0 {
            panic!("unlock failed: {}", Error::last_os_error());
        }

        let pts_name = ptsname(master_fd);
        if pts_name.is_null() {
            panic!("pts_name failed - is null");
        }

        let slave_path = CStr::from_ptr(pts_name as *const c_char)
            .to_str()
            .expect("Failed to convert pts_name to CStr");

        let master = std::fs::File::from_raw_fd(master_fd);
        Ok((master, slave_path))
    }
}
