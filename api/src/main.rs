use clap::Parser;
use std::io;
use tokio::{net::UdpSocket, sync::mpsc};
use tracing::{error, info};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Config {
    #[arg(long, env = "SERVER_URL")]
    server_url: String,

    #[arg(long, env = "ORG")]
    org: String,

    #[arg(long, env = "API_TOKEN")]
    api_token: String,

    #[arg(long, env = "BUCKET")]
    bucket: String,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::parse();

    let influx_client = infapi::InfClient::new(config.server_url, config.api_token, config.org)
        .precision(infapi::Precision::NS)
        .build();

    let sock = UdpSocket::bind("0.0.0.0:9090").await?;
    let (tx, mut rx) = mpsc::channel::<String>(1000);
    let bucket = config.bucket.clone();

    tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            let res = influx_client
                .write_point(bucket.to_string(), data.to_string())
                .await;
            match res {
                Ok(_) => {}
                Err(e) => error!("Error while sending data to influxdb: {:?}", e),
            }
        }
    });

    info!("Service is starting");
    loop {
        let mut buf = [0; 1024];
        let (len, _) = match sock.recv_from(&mut buf).await {
            Ok((len, addr)) => (len, addr),
            Err(e) => {
                error!("Error while getting data from UDP socket: {:?}", e);
                continue;
            }
        };

        // Extract data and send it to the influxdb service
        let data = match String::from_utf8(buf[..len].to_vec()) {
            Ok(s) => s,
            Err(e) => {
                error!("error getting data from the buffer: {}", e);
                continue;
            }
        };

        let _ = tx.send(data.clone()).await;
    }
}
