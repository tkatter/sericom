//! As of now, there is only one function, [`run_debug_output`], which is meant
//! to debug the data being received over the serial connection. In future
//! updates, this module is intended to be used for running tracing events with
//! the [`tracing`](https://docs.rs/tracing/latest/tracing/) crate.

use crate::serial_actor::SerialEvent;

/// This function is used for debugging the data that is sent from a device.
/// It will create a file "debug.txt" and print the data received from the device
/// as the actual bytes received along with the corresponding ascii characters.
///
/// Data is sent from the [`SerialActor`][crate::serial_actor::SerialActor] to this function in batches,
/// therefore a line written to "debug.txt" may look like this:
///
/// "\[04:41:27.550\] RX 9 bytes: \[0D, 0A, 53, 77, 69, 74, 63, 68\]... UTF8: ^M Switch#"
///
/// Each line will only print a maximum of 8 bytes, after 8 it will simply write "...".
pub async fn run_debug_output(mut rx: tokio::sync::broadcast::Receiver<SerialEvent>) {
    use std::io::{BufWriter, Write};
    use std::path::Path;

    let (write_tx, write_rx) = std::sync::mpsc::channel::<Vec<u8>>();
    let write_handle = tokio::task::spawn_blocking(move || {
        let path = Path::new("./debug.txt");
        let file = match std::fs::File::create(path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Failed to create file: {e}");
                return;
            }
        };
        let mut writer = BufWriter::with_capacity(48 * 1024, file);
        let mut last_flush = std::time::Instant::now();

        writeln!(writer, "Session started at: {}", chrono::Utc::now()).ok();
        while let Ok(data) = write_rx.recv() {
            let control_bytes_for_hex: Vec<u8> = data[..std::cmp::min(20, data.len())]
                .iter()
                .filter(|&&b| b.is_ascii_control()) // Use &&b for double reference
                .cloned() // Clone &u8 to u8
                .collect();
            writeln!(
                writer,
                "RX {} bytes: {:02X?}{} UTF8: {}",
                data.len(),
                control_bytes_for_hex,
                if data.len() > 20 { "..." } else { "" },
                String::from_utf8_lossy(&data)
            )
            .ok();
            // writeln!(
            //     writer,
            //     "[{}] RX {} bytes: {:02X?}{} UTF8: {}",
            //     chrono::Utc::now().format("%H:%M:%S%.3f"),
            //     data.len(),
            //     &data[..std::cmp::min(20, data.len())],
            //     if data.len() > 10 { "..." } else { "" },
            //     String::from_utf8_lossy(&data)
            // )
            // .ok();

            let now = std::time::Instant::now();
            if now.duration_since(last_flush) > std::time::Duration::from_millis(100)
                || writer.buffer().len() > 32 * 1024
            {
                let _ = writer.flush();
                last_flush = now;
            }
        }
        let _ = writer.flush();
    });

    let data_streamer = tokio::spawn(async move {
        let mut write_buf = Vec::with_capacity(4096);
        let mut batch_timer = tokio::time::interval(tokio::time::Duration::from_millis(200));

        loop {
            tokio::select! {
                event = rx.recv() => {
                    match event {
                        Ok(SerialEvent::Data(data)) => {
                            write_buf.extend_from_slice(&data);
                            if write_buf.len() >= 4096 && write_tx.send(std::mem::take(&mut write_buf)).is_err() {
                                    break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            eprintln!("File writer lagged, skipped {skipped} messages");
                            continue; // Don't break on lag
                        }
                        _ => break,
                    }
                }
                _ = batch_timer.tick() => {
                    if !write_buf.is_empty() && write_tx.send(std::mem::take(&mut write_buf)).is_err() {
                            break;
                    }
                }
            }
        }
        if !write_buf.is_empty() {
            let _ = write_tx.send(std::mem::take(&mut write_buf));
        }
        drop(write_tx);
    });

    let _ = data_streamer.await;
    let _ = write_handle.await;
}
