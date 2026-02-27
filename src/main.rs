mod beast;
mod co2;
mod db;
mod icao_country;
mod json_output;
mod readsb;
mod tracker;

use clap::Parser;
use adsb_deku::deku::DekuContainerRead;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{self, Duration};
use tracing::{error, info, warn};

/// rs1090 — ADS-B CO₂ tracker (future tar1090 replacement)
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Beast TCP source address (capture container)
    #[arg(
        short = 'b',
        long,
        default_value = "127.0.0.1:30005",
        env = "BEAST_SOURCE"
    )]
    beast_source: String,

    /// Output directory for aircraft.json / receiver.json
    #[arg(
        short = 'o',
        long,
        default_value = "/run/readsb",
        env = "JSON_OUTPUT_DIR"
    )]
    output_dir: PathBuf,

    /// SQLite database path
    #[arg(
        short = 'd',
        long,
        default_value = "/var/lib/co2tracker/co2.db",
        env = "CO2_DB"
    )]
    db_path: PathBuf,

    /// HTTP listen address for the JSON API
    #[arg(
        short = 'l',
        long,
        default_value = "0.0.0.0:8181",
        env = "RS1090_LISTEN"
    )]
    listen: String,

    /// Receiver latitude (for map center and range rings)
    #[arg(long, env = "RECEIVER_LAT")]
    receiver_lat: Option<f64>,

    /// Receiver longitude
    #[arg(long, env = "RECEIVER_LON")]
    receiver_lon: Option<f64>,

    /// Maximum seconds since last position before marking stale
    #[arg(long, default_value_t = 120)]
    stale_secs: u64,

    /// Seconds between reconnection attempts to Beast source
    #[arg(long, default_value_t = 5)]
    reconnect_interval: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rs1090=info".into()),
        )
        .init();

    let args = Args::parse();

    info!(
        beast_source = %args.beast_source,
        output_dir = %args.output_dir.display(),
        db = %args.db_path.display(),
        listen = %args.listen,
        "rs1090 starting"
    );

    // Ensure directories exist
    std::fs::create_dir_all(&args.output_dir)?;
    if let Some(parent) = args.db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Open database
    let database = db::Database::open(&args.db_path)?;
    info!("database ready");
    let _db = Arc::new(Mutex::new(database));

    // Write receiver.json once
    json_output::write_receiver_json(
        &args.output_dir,
        args.receiver_lat,
        args.receiver_lon,
    )?;
    info!("receiver.json written");

    // Shared tracker state
    let tracker = Arc::new(Mutex::new(tracker::Tracker::new(
        args.receiver_lat,
        args.receiver_lon,
    )));

    // Spawn the JSON writer task (writes aircraft.json every second)
    let json_tracker = Arc::clone(&tracker);
    let json_dir = args.output_dir.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            let mut t = json_tracker.lock().await;
            t.update_ages();
            t.expire_stale();
            if let Err(e) = json_output::write_aircraft_json(&json_dir, &t) {
                warn!("failed to write aircraft.json: {e}");
            }
        }
    });

    // Beast reader loop with reconnection
    let beast_addr = args.beast_source.clone();
    let beast_tracker = Arc::clone(&tracker);
    let reconnect_secs = args.reconnect_interval;

    loop {
        info!(addr = %beast_addr, "connecting to Beast source...");
        match beast::BeastReader::connect(&beast_addr).await {
            Ok(mut reader) => {
                info!("connected to Beast source");
                loop {
                    match reader.next_frame().await {
                        Ok(frame) => {
                            // Decode the Mode S message
                            match adsb_deku::Frame::from_bytes((&frame.message, 0)) {
                                Ok((_, decoded)) => {
                                    let mut t = beast_tracker.lock().await;
                                    t.update(&decoded, Some(frame.signal));
                                }
                                Err(_) => {
                                    // Invalid/garbled frame — common, just skip
                                }
                            }
                        }
                        Err(beast::BeastError::ConnectionClosed) => {
                            warn!("Beast connection closed");
                            break;
                        }
                        Err(e) => {
                            error!("Beast read error: {e}");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                warn!("failed to connect to Beast source: {e}");
            }
        }

        info!(secs = reconnect_secs, "reconnecting in...");
        time::sleep(Duration::from_secs(reconnect_secs)).await;
    }
}
