use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream, UdpSocket};

const LISTEN_ADDR: &str = "127.0.0.1:5335";
const UPSTREAM_ADDR: &str = "127.0.0.1:5336";
const MAX_UDP_PACKET_SIZE: usize = 65_535;

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("laliga-dns-evade v{}", env!("CARGO_PKG_VERSION"));
    println!("Listening on {LISTEN_ADDR} (UDP/TCP)");
    println!("Upstream: {UPSTREAM_ADDR}");

    let udp_task = tokio::spawn(run_udp_proxy());
    let tcp_task = tokio::spawn(run_tcp_proxy());

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

async fn run_udp_proxy() -> io::Result<()> {
    let listener = Arc::new(UdpSocket::bind(LISTEN_ADDR).await?);
    let mut buffer = vec![0_u8; MAX_UDP_PACKET_SIZE];

    loop {
        let (length, client_addr) = listener.recv_from(&mut buffer).await?;
        let request = buffer[..length].to_vec();
        let client_socket = Arc::clone(&listener);

        tokio::spawn(async move {
            if let Err(error) = forward_udp(client_socket, request, client_addr).await {
                eprintln!("UDP forwarding error for {client_addr}: {error}");
            }
        });
    }
}

async fn forward_udp(
    client_socket: Arc<UdpSocket>,
    request: Vec<u8>,
    client_addr: SocketAddr,
) -> io::Result<()> {
    let upstream = UdpSocket::bind("127.0.0.1:0").await?;
    upstream.connect(UPSTREAM_ADDR).await?;
    upstream.send(&request).await?;

    let mut response = vec![0_u8; MAX_UDP_PACKET_SIZE];
    let length = upstream.recv(&mut response).await?;

    client_socket
        .send_to(&response[..length], client_addr)
        .await?;

    Ok(())
}

async fn run_tcp_proxy() -> io::Result<()> {
    let listener = TcpListener::bind(LISTEN_ADDR).await?;

    loop {
        let (client, client_addr) = listener.accept().await?;

        tokio::spawn(async move {
            if let Err(error) = forward_tcp(client).await {
                eprintln!("TCP forwarding error for {client_addr}: {error}");
            }
        });
    }
}

async fn forward_tcp(mut client: TcpStream) -> io::Result<()> {
    let mut upstream = TcpStream::connect(UPSTREAM_ADDR).await?;

    copy_bidirectional(&mut client, &mut upstream).await?;

    Ok(())
}
