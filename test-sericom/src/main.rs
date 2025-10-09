use std::{
    io::Write,
    process::{Child, Command},
    thread,
    time::Duration,
};

mod pts;
use pts::get_pts_pair;

fn main() -> std::io::Result<()> {
    let seq = b"\x1B[12G";
    let n = parse_number_field_bytes(seq);
    println!("{n:?}"); // Some(12)

    let seq = b"\x1B[46G";
    let n = parse_number_field_bytes(seq);
    println!("{n:?}"); // Some(12)

    for b in b"0123456789" {
        println!("{b} '{}' → {}", *b as char, b - b'0');
    }
    for b in b"0123456789" {
        println!("{}: {:08b}  digit = {}", *b as char, b, b & 0x0F);
    }

    let sub = b'5' - b'0';
    let and = b'5' & 0x0F;
    println!("SUB: {:08b}, AND: {:08b}", sub, and);
    // let (mut master, slave) = get_pts_pair()?;
    // println!("Got slave: {slave}");
    //
    // let mut seri_guard = ChildGuard {
    //     child: Some(
    //         Command::new("/home/thomas/.cargo/bin/sericom")
    //             .arg(slave)
    //             .spawn()
    //             .expect("Failed to start 'sericom'"),
    //     ),
    // };
    //
    // master.write_all(b"Hello from simulated serial!\r\n")?;
    // master.flush()?;
    //
    // thread::sleep(Duration::from_secs(2));
    //
    // master.write_all(b"\x1B[2J")?;
    // master.write_all(b"\x1B[H")?;
    // master.write_all(b"Ready>\r\n")?;
    // master.flush()?;
    //
    // thread::sleep(Duration::from_secs(5));
    // seri_guard.wait();
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

fn parse_number_field_bytes(seq: &[u8]) -> Option<u32> {
    let body = &seq[2..seq.len() - 1];
    if body.is_empty() || !body.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }

    Some(
        body.iter()
            .fold(0u32, |acc, b| acc * 10 + (b & 0x0F) as u32)
    )
}

#[test]
fn test_parse() {
    let seq = b"\x1B[12G";
    let n = parse_number_field_bytes(seq);
    println!("{n:?}"); // Some(12)
}
