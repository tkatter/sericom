use std::{
    fs::File,
    io::{self, Write},
};

use clap::{Parser, Subcommand};
use serial2_tokio::SerialPort;

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
    /// Lists all valid baud rates
    ListBauds,
    /// Lists all available serial ports
    ListPorts {
        /// [DEFAULT] - Streams the serial output to stdout
        #[arg(short, long, default_value_t = true)]
        stream: bool,
        /// Writes the serial output to the specified file
        #[arg(short, long)]
        file: Option<String>,
    },
    /// Gets the settings for a serial port
    ListSettings {
        /// Specify the baud rate for the serial connection - REQUIRED IF '--keep-settings' NOT
        /// PRESENT
        #[arg(short, long, value_parser = valid_baud_rate, required_unless_present = "keep_settings")]
        baud: Option<u32>,
        /// Path to the port to open
        #[arg(short, long)]
        port: String,
        /// Keeps the existing serial port configuration
        #[arg(short, long)]
        keep_settings: bool,
        /// [DEFAULT] - Streams the serial output to stdout
        #[arg(short, long, default_value_t = true)]
        stream: bool,
        /// Writes the serial output to the specified file
        #[arg(short, long)]
        file: Option<String>,
    },
    /// Opens a port and reads the recieved data
    ReadPort {
        /// Specify the baud rate for the serial connection - REQUIRED IF '--keep-settings' NOT
        /// PRESENT
        #[arg(short, long, value_parser = valid_baud_rate, required_unless_present = "keep_settings")]
        baud: Option<u32>,
        /// Path to the port to open
        #[arg(short, long)]
        port: String,
        /// Keeps the existing serial port configuration
        #[arg(short, long)]
        keep_settings: bool
    },
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    match cli.command {
        Commands::ListBauds => {
            write!(handle, "Valid baud rates:\r\n")?;
            for baud in serial2_tokio::COMMON_BAUD_RATES {
                write!(handle, "{baud}\r\n")?;
            }
        }
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
            } else if stream {
                list_serial_ports(Box::new(handle))?
            }
        }
        Commands::ListSettings { baud, port, keep_settings, stream, file } => {
            if let Some(file) = file {
                let path = std::path::Path::new(&file);
                let mut file_handle = File::options().append(true).create(true).open(path)?;
                if path.metadata()?.len() == 0 {
                    write!(file_handle, "TIMESTAMP: {}\r\nPORT: {port}\r\n", chrono::Utc::now())?;
                } else {
                    write!(file_handle, "\r\nTIMESTAMP: {}\r\nPORT: {port}\r\n", chrono::Utc::now())?;
                }
                get_settings(Box::new(file_handle), baud, &port, keep_settings)?;
            } else if stream {
                get_settings(Box::new(handle), baud, &port, keep_settings)?;
            }
        }
        Commands::ReadPort { baud, port, keep_settings}
        => {
            let mut buff: Vec<u8> = Vec::new();
            // let mut buff: [u8; 255] = [0; 255];
            let con = open_port(baud, &port, keep_settings)?;
            stream_to_stdout(&mut buff, con).await?;
        }
    }
    Ok(())
}

fn open_port(baud: Option<u32>, port: &str, keep_settings: bool) -> io::Result<SerialPort> {
    let con = if keep_settings {
        SerialPort::open(port, serial2_tokio::KeepSettings)?
    } else if let Some(baud) = baud {
        SerialPort::open(port, baud)?
    } else {
        unreachable!()
    };
    Ok(con)
}

#[allow(clippy::needless_lifetimes)]
async fn stream_to_stdout<'a>(buff: &'a mut Vec<u8>, con: SerialPort) -> io::Result<()> {
    let read_to_buff = async |buff: &'a mut Vec<u8>, con: SerialPort| -> io::Result<&'a [u8]> {
        con.read(buff).await?;
        Ok(buff.as_slice())
    };
    let mut reader = read_to_buff(buff, con).await?;
    let mut writer = tokio::io::stdout();

    tokio::io::copy(&mut reader, &mut writer).await?;
    Ok(())
}

fn get_settings(mut handle: Box<dyn io::Write>, baud: Option<u32>, port: &str, keep_settings: bool) -> Result<(), io::Error> {
    // https://www.contec.com/support/basic-knowledge/daq-control/serial-communicatin/

    // NOTE: CUSTOM SETTINGS CLOSURE
    // let settings = |mut s: serial2_tokio::Settings| -> std::io::Result<serial2_tokio::Settings> {
    //     s.set_raw();
    //     s.set_baud_rate(*baud)?;
    //     s.set_char_size(serial2_tokio::CharSize::Bits8);
    //     s.set_stop_bits(serial2_tokio::StopBits::Two);
    //     Ok(s)
    // };
    // let con = serial2_tokio::SerialPort::open(con, settings)?;

    let con = open_port(baud, port, keep_settings)?;
    let settings = con.get_configuration()?;

    let b = settings.get_baud_rate()?;
    let c = settings.get_char_size()?;
    let s = settings.get_stop_bits()?;
    let p = settings.get_parity()?;
    let f = settings.get_flow_control()?;

    let cts = con.read_cts()?;
    let dsr = con.read_dsr()?;
    let ri = con.read_ri()?;
    let cd = con.read_cd()?;

    write!(handle, "Baud rate: {b}\r\n")?;
    write!(handle, "Char size: {c}\r\n")?;
    write!(handle, "Stop bits: {s}\r\n")?;
    write!(handle, "Parity mechanism: {p}\r\n")?;
    write!(handle, "Flow control: {f}\r\n")?;
    write!(handle, "Clear To Send line: {cts}\r\n")?;
    write!(handle, "Data Set Ready line: {dsr}\r\n")?;
    write!(handle, "Ring Indicator line: {ri}\r\n")?;
    write!(handle, "Carrier Detect line: {cd}\r\n")?;

    Ok(())
}

fn list_serial_ports(mut handle: Box<dyn io::Write>) -> Result<(), io::Error> {
    let ports = SerialPort::available_ports()?;
    for path in ports {
        if let Some(path) = path.to_str() {
            let line = [path, "\r\n"].concat();
            handle.write(line.as_bytes())?
        } else {
            continue;
        };
    }
    Ok(())
}

fn valid_baud_rate(s: &str) -> Result<u32, String> {
    let baud: u32 = s
        .parse()
        .map_err(|_| format!("`{s}` isn't a valid baud rate"))?;
    if serial2_tokio::COMMON_BAUD_RATES.contains(&baud) {
        Ok(baud)
    } else {
        Err(format!(
            "'{}' is not a valid baud rate; valid baud rates include {:?}",
            baud,
            serial2_tokio::COMMON_BAUD_RATES
        ))
    }
}
