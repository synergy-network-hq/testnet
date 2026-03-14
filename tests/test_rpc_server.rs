use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
use synergy_testbeta::rpc;

#[test]
fn test_rpc_server() {
    // Check if port 8545 is already in use
    if TcpListener::bind("0.0.0.0:8545").is_ok() {
        // If the port is free, start the RPC server in a separate thread
        thread::spawn(|| {
            rpc::start_rpc_server();
        });

        // Wait a few seconds for the server to fully start
        thread::sleep(Duration::from_secs(3));
    } else {
        println!("RPC server is already running, skipping server startup in test.");
    }

    // Attempt to connect to the RPC server
    let mut stream = TcpStream::connect("0.0.0.0:8545").expect("Failed to connect to RPC server");

    // Send a dummy request to test response handling
    let request = b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n";
    stream.write_all(request).expect("Failed to send request");

    // Read the response
    let mut buffer = [0; 1024];
    let bytes_read = stream.read(&mut buffer).expect("Failed to read response");

    // Ensure we got a response and check for HTTP 200 OK
    assert!(bytes_read > 0);
    assert!(String::from_utf8_lossy(&buffer).contains("200 OK"));

    println!("RPC server test successfully completed.");
}
