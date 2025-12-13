use std::{
    io::Write,
    process::{Child, Command},
    thread,
    time::Duration,
};

#[cfg(unix)]
mod pts;
#[cfg(unix)]
use pts::get_pts_pair;

fn main() -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let (mut master, slave) = get_pts_pair()?;
        println!("Got slave: {slave}");

        let mut seri_guard = ChildGuard {
            child: Some(
                Command::new("/home/thomas/.cargo/bin/sericom")
                    .arg(slave)
                    .spawn()
                    .expect("Failed to start 'sericom'"),
            ),
        };

        master.write_all(b"Hello from simulated serial!\r\n")?;
        master.flush()?;

        thread::sleep(Duration::from_secs(2));

        master.write_all(b"\x1B[2J")?;
        master.write_all(b"\x1B[H")?;
        master.write_all(b"Ready>\r\n")?;
        master.flush()?;

        thread::sleep(Duration::from_secs(5));
        seri_guard.wait();
    }
    Ok(())
}

struct ChildGuard {
    child: Option<Child>,
}

impl ChildGuard {
    fn wait(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.wait();
        }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

#[test]
fn test_sub() {
    let seq = [0x1B, b'[', b'6', b'n'];
    let body = &seq[2..seq.len() - 1];

    println!("{:?}", body);
    assert_eq!(body, [b'6']);
    // assert!(body.is_empty());
    assert!(body.len() == 1);
}
