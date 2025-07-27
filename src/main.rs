use std::{
    fs::File,
    io::{self, Write},
};

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "SerialTool", version, about, long_about = None)]
#[command(next_line_help = true)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Lists all available serial ports
    ListPorts {
        /// [DEFAULT] - Streams the serial output to stdout
        #[arg(short, long, default_value_t = true)]
        stream: bool,
        /// Writes the serial output to the specified file
        #[arg(short, long)]
        file: Option<String>,
    },
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let stdout = io::stdout();
    let handle = stdout.lock();

    match &cli.command {
        Commands::ListPorts { stream, file } => {
            if let Some(file) = file {
                let path = std::path::Path::new(&file);
                let mut file_handle = File::options().append(true).create(true).open(path)?;
                if path.metadata()?.len() == 0 {
                    write!(file_handle, "UTC: {}\r\n", chrono::Utc::now())?;
                } else {
                    write!(file_handle, "\r\nUTC: {}\r\n", chrono::Utc::now())?;
                }
                list_serial_ports(Box::new(file_handle))?
            } else if *stream {
                list_serial_ports(Box::new(handle))?
            }
        }
    }

    Ok(())
}

fn list_serial_ports(mut handle: Box<dyn io::Write>) -> Result<(), io::Error> {
    if let Ok(ports) = serial2_tokio::SerialPort::available_ports() {
        for path in ports {
            if let Some(path) = path.to_str() {
                let line = [path, "\r\n"].concat();
                handle.write(line.as_bytes())?
            } else {
                continue;
            };
        }
    };
    Ok(())
}
