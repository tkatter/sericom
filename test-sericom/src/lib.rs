#[cfg(unix)]
pub use pts::get_pts_pair;

#[cfg(unix)]
mod pts {
    use std::{ffi::CStr, fs::File, os::fd::FromRawFd};

    // Needed constants (from fcntl.h)
    const O_RDWR: libc::c_int = libc::O_RDWR;
    const O_NOCTTY: libc::c_int = libc::O_NOCTTY;

    /// # Example
    /// ```
    /// let (mut master, slave) = get_pts_pair()?;
    /// println!("Got slave: {slave}");
    ///
    /// master.write_all(b"Hello from simulated serial!\r\n")?;
    /// master.flush()?;
    /// ```
    pub fn get_pts_pair() -> std::io::Result<(File, &'static str)> {
        use std::io::Error;

        unsafe {
            let master_fd: libc::c_int = libc::posix_openpt(O_RDWR | O_NOCTTY);
            if master_fd < 0 {
                let e = Error::last_os_error();
                tracing::error!("posix_openpt failed: {e}");
                return Err(e);
            }

            if libc::grantpt(master_fd) != 0 {
                let e = Error::last_os_error();
                tracing::error!("grantpt failed: {e}");
                libc::close(master_fd);
                return Err(e);
            }
            if libc::unlockpt(master_fd) != 0 {
                let e = Error::last_os_error();
                tracing::error!("unlockpt failed: {e}");
                libc::close(master_fd);
                return Err(e);
            }

            let pts_name = libc::ptsname(master_fd);
            if pts_name.is_null() {
                let e = Error::last_os_error();
                tracing::error!("pts_name failed - is null: {e}");
                libc::close(master_fd);
                return Err(e);
            }

            let slave_path = CStr::from_ptr(pts_name as *const libc::c_char)
                .to_str()
                .expect("Failed to convert pts_name to CStr");

            set_baud_raw(master_fd).inspect_err(|_| {
                libc::close(master_fd);
            })?;

            let slave_fd = libc::open(pts_name, O_RDWR | O_NOCTTY);
            if slave_fd < 0 {
                let e = Error::last_os_error();
                tracing::error!("failed to open the slave: {e}");
                libc::close(master_fd);
                return Err(e);
            }

            set_baud_raw(slave_fd).inspect_err(|_| {
                libc::close(slave_fd);
                libc::close(master_fd);
            })?;
            libc::close(slave_fd);

            let master = std::fs::File::from_raw_fd(master_fd);
            tracing::debug!(%slave_path);
            Ok((master, slave_path))
        }
    }

    fn set_baud_raw(fd: libc::c_int) -> std::io::Result<()> {
        use std::io::Error;

        let mut tio: libc::termios = unsafe {
            let mut tio: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut tio) != 0 {
                let e = Error::last_os_error();
                tracing::error!("tcgetattr failed: {e}");
                return Err(e);
            }
            tio
        };

        // set input/output speeds
        unsafe {
            if libc::cfsetispeed(&mut tio, libc::B9600) != 0
                || libc::cfsetospeed(&mut tio, libc::B9600) != 0
            {
                let e = Error::last_os_error();
                tracing::error!("failed to set input/output speeds: {e}");
                return Err(e);
            }
        }

        // a minimal “raw” setup: 8 N 1, no echo / canonical
        tio.c_iflag = 0;
        tio.c_oflag = 0;
        tio.c_cflag = libc::CS8 | libc::CREAD | libc::CLOCAL;
        tio.c_lflag = 0;
        tio.c_cc[libc::VMIN] = 1;
        tio.c_cc[libc::VTIME] = 0;

        unsafe {
            if libc::tcsetattr(fd, libc::TCSANOW, &tio) != 0 {
                let e = Error::last_os_error();
                tracing::error!("tcsetattr failed: {e}");
                return Err(e);
            }
        }

        Ok(())
    }
}
