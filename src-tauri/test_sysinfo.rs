use sysinfo::Networks;
#[tokio::main]
async fn main() {
    let mut networks = Networks::new_with_refreshed_list();
    let mut last_rx: u64 = networks.iter().map(|(_, n)| n.total_received()).sum();
    for _ in 0..5 {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        networks.refresh(true);
        let current_rx: u64 = networks.iter().map(|(_, n)| n.total_received()).sum();
        let rx_speed = current_rx.saturating_sub(last_rx);
        println!("current: {}, last: {}, speed: {}", current_rx, last_rx, rx_speed);
        last_rx = current_rx;
    }
}
