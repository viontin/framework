pub fn set_connection_timeout(stream: &std::net::TcpStream, seconds: u64) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(seconds)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(seconds)));
}
