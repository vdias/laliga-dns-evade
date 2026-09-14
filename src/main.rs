mod networks;
mod blocklist;
mod dns;
use dns::{clear_authenticated_data, extract_address_records, rewrite_address_record};
use blocklist::Blocklist;
use networks::{find_evasive_address, NetworkList};
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};

const BLOCKLIST_PATH: &str = "/var/lib/laliga-dns-evade/blocked-any.txt";
const CLOUDFLARE_V4_PATH: &str = "/var/lib/laliga-dns-evade/cloudflare-v4.txt";
const CLOUDFLARE_V6_PATH: &str = "/var/lib/laliga-dns-evade/cloudflare-v6.txt";
const LISTEN_ADDR: &str = "127.0.0.1:5335";
const UPSTREAM_ADDR: &str = "127.0.0.1:5336";
const MAX_UDP_PACKET_SIZE: usize = 65_535;

#[tokio::main]
async fn main() -> io::Result<()> {
    let blocklist = Arc::new(
        Blocklist::load(BLOCKLIST_PATH)
            .map_err(io::Error::other)?
    );

    println!("Loaded {} blocked IP addresses", blocklist.len());

    let cloudflare_v4 = Arc::new(
        NetworkList::load(CLOUDFLARE_V4_PATH)
            .map_err(io::Error::other)?
    );

    let cloudflare_v6 = Arc::new(
        NetworkList::load(CLOUDFLARE_V6_PATH)
            .map_err(io::Error::other)?
    );

    println!(
        "Loaded {} Cloudflare IPv4 prefixes and {} IPv6 prefixes",
        cloudflare_v4.len(),
        cloudflare_v6.len()
    );

    println!("laliga-dns-evade v{}", env!("CARGO_PKG_VERSION"));
    println!("Listening on {LISTEN_ADDR} (UDP/TCP)");
    println!("Upstream: {UPSTREAM_ADDR}");

    let udp_task = tokio::spawn(run_udp_proxy(
        Arc::clone(&blocklist),
        Arc::clone(&cloudflare_v4),
        Arc::clone(&cloudflare_v6),
    ));
    let tcp_task = tokio::spawn(run_tcp_proxy(
        Arc::clone(&blocklist),
        Arc::clone(&cloudflare_v4),
        Arc::clone(&cloudflare_v6),
    ));

    tokio::select! {
        result = udp_task => {
            match result {
                Ok(result) => result,
                Err(error) => Err(io::Error::other(format!("UDP task failed: {error}"))),
            }
        }
        result = tcp_task => {
            match result {
                Ok(result) => result,
                Err(error) => Err(io::Error::other(format!("TCP task failed: {error}"))),
            }
        }
        _ = tokio::signal::ctrl_c() => {
            println!("Shutdown requested");
            Ok(())
        }
    }
}

async fn run_udp_proxy(
    blocklist: Arc<Blocklist>,
    cloudflare_v4: Arc<NetworkList>,
    cloudflare_v6: Arc<NetworkList>,
) -> io::Result<()> {
    let listener = Arc::new(UdpSocket::bind(LISTEN_ADDR).await?);
    let mut buffer = vec![0_u8; MAX_UDP_PACKET_SIZE];

    loop {
        let (length, client_addr) = listener.recv_from(&mut buffer).await?;
        let request = buffer[..length].to_vec();
        let client_socket = Arc::clone(&listener);
        let blocklist = Arc::clone(&blocklist);
        let cloudflare_v4 = Arc::clone(&cloudflare_v4);
        let cloudflare_v6 = Arc::clone(&cloudflare_v6);

        tokio::spawn(async move {
            if let Err(error) = forward_udp(
                client_socket,
                request,
                client_addr,
                blocklist,
                cloudflare_v4,
                cloudflare_v6,
            )
            .await
            {
                eprintln!("UDP forwarding error for {client_addr}: {error}");
            }
        });
    }
}

async fn forward_udp(
    client_socket: Arc<UdpSocket>,
    request: Vec<u8>,
    client_addr: SocketAddr,
    blocklist: Arc<Blocklist>,
    cloudflare_v4: Arc<NetworkList>,
    cloudflare_v6: Arc<NetworkList>,
) -> io::Result<()> {
    let upstream = UdpSocket::bind("127.0.0.1:0").await?;
    upstream.connect(UPSTREAM_ADDR).await?;
    upstream.send(&request).await?;

    let mut response = vec![0_u8; MAX_UDP_PACKET_SIZE];
    let length = upstream.recv(&mut response).await?;

    match extract_address_records(&response[..length]) {
        Ok(records) => {
            for record in records {
                let address = record.address();

                if !blocklist.contains(&address) {
                    continue;
                }

                let networks = match address {
                    IpAddr::V4(_) => cloudflare_v4.as_ref(),
                    IpAddr::V6(_) => cloudflare_v6.as_ref(),
                };

                if !networks.contains(&address) {
                    println!("BLOCKED NON-CLOUDFLARE address detected: {address}");
                    continue;
                }

                match find_evasive_address(&address, networks, blocklist.as_ref()) {
                    Some(replacement) => {
                        rewrite_address_record(
                            &mut response[..length],
                            &record,
                            replacement,
                        )
                        .map_err(io::Error::other)?;

                        clear_authenticated_data(&mut response[..length])
                            .map_err(io::Error::other)?;

                        println!(
                            "Rewritten blocked Cloudflare address: {address} -> {replacement}"
                        );
                    }
                    None => {
                        println!(
                            "Blocked Cloudflare address detected but no safe replacement found: {address}"
                        );
                    }
                }
            }
        }
        Err(error) => {
            eprintln!("Failed to inspect UDP DNS response: {error}");
        }
    }

    client_socket
        .send_to(&response[..length], client_addr)
        .await?;

    Ok(())
}

async fn run_tcp_proxy(
    blocklist: Arc<Blocklist>,
    cloudflare_v4: Arc<NetworkList>,
    cloudflare_v6: Arc<NetworkList>,
) -> io::Result<()> {
    let listener = TcpListener::bind(LISTEN_ADDR).await?;

    loop {
        let (client, client_addr) = listener.accept().await?;
        let blocklist = Arc::clone(&blocklist);
        let cloudflare_v4 = Arc::clone(&cloudflare_v4);
        let cloudflare_v6 = Arc::clone(&cloudflare_v6);

        tokio::spawn(async move {
            if let Err(error) = forward_tcp(
                client,
                blocklist,
                cloudflare_v4,
                cloudflare_v6,
            )
            .await
            {
                eprintln!("TCP forwarding error for {client_addr}: {error}");
            }
        });
    }
}

async fn forward_tcp(
    mut client: TcpStream,
    blocklist: Arc<Blocklist>,
    cloudflare_v4: Arc<NetworkList>,
    cloudflare_v6: Arc<NetworkList>,
) -> io::Result<()> {
    let mut upstream = TcpStream::connect(UPSTREAM_ADDR).await?;

    loop {
        let mut request_length_bytes = [0_u8; 2];

        match client.read_exact(&mut request_length_bytes).await {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => {
                return Ok(());
            }
            Err(error) => return Err(error),
        }

        let request_length = u16::from_be_bytes(request_length_bytes) as usize;
        let mut request = vec![0_u8; request_length];

        client.read_exact(&mut request).await?;

        upstream.write_all(&request_length_bytes).await?;
        upstream.write_all(&request).await?;

        let mut response_length_bytes = [0_u8; 2];
        upstream.read_exact(&mut response_length_bytes).await?;

        let response_length =
            u16::from_be_bytes(response_length_bytes) as usize;

        let mut response = vec![0_u8; response_length];
        upstream.read_exact(&mut response).await?;

        match extract_address_records(&response) {
            Ok(records) => {
                for record in records {
                    let address = record.address();

                    if !blocklist.contains(&address) {
                        continue;
                    }

                    let networks = match address {
                        IpAddr::V4(_) => cloudflare_v4.as_ref(),
                        IpAddr::V6(_) => cloudflare_v6.as_ref(),
                    };

                    if !networks.contains(&address) {
                        println!(
                            "BLOCKED NON-CLOUDFLARE address detected over TCP: {address}"
                        );
                        continue;
                    }

                    match find_evasive_address(
                        &address,
                        networks,
                        blocklist.as_ref(),
                    ) {
                        Some(replacement) => {
                            rewrite_address_record(
                                &mut response,
                                &record,
                                replacement,
                            )
                            .map_err(io::Error::other)?;

                            clear_authenticated_data(&mut response)
                                .map_err(io::Error::other)?;

                            println!(
                                "Rewritten blocked Cloudflare address over TCP:                                  {address} -> {replacement}"
                            );
                        }
                        None => {
                            println!(
                                "Blocked Cloudflare address detected over TCP                                  but no safe replacement found: {address}"
                            );
                        }
                    }
                }
            }
            Err(error) => {
                eprintln!(
                    "Failed to inspect TCP DNS response: {error}"
                );
            }
        }

        client.write_all(&response_length_bytes).await?;
        client.write_all(&response).await?;
    }
}
