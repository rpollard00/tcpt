use ctrlc;
use std::fmt;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    hostname: Option<String>,

    #[arg(default_value_t = 443)]
    port: u16,

    #[arg(short, long, default_value_t = 1000)]
    interval: u64,
}

struct DurationDisplay(Duration);

struct HostAddr {
    addr: SocketAddr,
    hostname: String,
}

impl From<Duration> for DurationDisplay {
    fn from(duration: Duration) -> Self {
        DurationDisplay(duration)
    }
}

impl fmt::Display for DurationDisplay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ms = self.0.as_millis();
        write!(f, "{} ms", ms)
    }
}

fn lookup_ip(hostname: &str, port: u16) -> Result<SocketAddr, &str> {
    let addresses = format!("{}:{}", hostname, port).to_socket_addrs();
    match addresses {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                Ok(addr)
            } else {
                Err("No addresses found for hostname")
            }
        }
        Err(_) => Err("Unable to resolve hostname"),
    }
}

fn get_socket_addr(hostname: &str, port: u16) -> Result<HostAddr, &str> {
    let address = lookup_ip(hostname, port);
    match address {
        Ok(addr) => Ok(HostAddr {
            addr,
            hostname: String::from(hostname),
        }),
        Err(e) => Err(e),
    }
}

fn tcp_connect(address: &HostAddr, seq: &u128) -> Result<u128, bool> {
    let timeout = Duration::from_secs(3);
    let start = Instant::now();

    match TcpStream::connect_timeout(&address.addr, timeout) {
        Ok(_) => {
            let duration = start.elapsed();
            let duration = DurationDisplay::from(duration);
            println!(
                "Connected to [{}] {} seq={} time={}",
                address.addr, address.hostname, seq, duration
            );
            Ok(duration.0.as_millis())
        }
        Err(_) => {
            println!(
                "Failed to connect to [{}] {}...",
                address.addr, address.hostname
            );
            Err(false)
        }
    }
}

fn execute_tcpping() {
    let cli = Cli::parse();
    let port = cli.port;
    let interval = cli.interval;

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || r.store(false, Ordering::SeqCst))
        .expect("Error setting Ctrl-C Handler");

    let mut avg: u128 = 0;
    let mut min: u128 = u128::MAX;
    let mut max: u128 = 0;
    let mut lost: u128 = 0;

    let mut seq: u128 = 1;
    if let Some(hostname) = cli.hostname {
        if let Ok(sockaddr) = get_socket_addr(&hostname, port) {
            while running.load(Ordering::SeqCst) {
                let rtt = tcp_connect(&sockaddr, &seq);
                match rtt {
                    Ok(t) => {
                        avg = (avg * (seq - 1) + t) / seq;
                        min = min.min(t);
                        max = max.max(t);
                    }
                    Err(_) => {
                        lost += 1;
                        avg = avg * (seq - 1) / seq;
                    }
                };
                sleep(Duration::from_millis(interval));
                seq += 1;
            }
        }
    }
    print!(
        "\nTotal: {} Avg: {}ms Min: {}ms Max: {}ms Lost: {}\n",
        seq, avg, min, max, lost
    );
}

fn main() {
    execute_tcpping();
}
