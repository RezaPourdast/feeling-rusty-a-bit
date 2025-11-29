use ping;
use std::time::Instant;

fn main() {
    loop {
        println!("{}", get_ping());
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn get_ping() -> String {
    let target_ip = "8.8.8.8".parse::<std::net::IpAddr>().expect("invalid IP");

    let mut p = ping::new(target_ip);
    p.timeout(std::time::Duration::from_secs(2)).ttl(128);

    let start = Instant::now();

    match p.send() {
        Ok(_) => {
            let rtt_ms = start.elapsed().as_millis();
            format!("{}", rtt_ms)
        }
        Err(e) => format!("Ping failed: {}", e),
    }
}
