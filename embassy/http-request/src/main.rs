#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::dma;
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::InterruptHandler;

// defmt Logging
use defmt::info;
use defmt_rtt as _;

use panic_probe as _;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>;
});

const WIFI_NETWORK: &str = env!("SSID");
const WIFI_PASSWORD: &str = env!("PASSWORD");

mod http_client;
mod wifi;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    info!("Initializing the program");

    let (net_device, mut control) = wifi::init_cyw43(
        spawner, p.PIO0, p.DMA_CH0, p.PIN_23, p.PIN_24, p.PIN_25, p.PIN_29,
    )
    .await;

    let mut rng = RoscRng;
    let seed = rng.next_u64();

    let stack = wifi::init_net_stack(spawner, net_device, seed);

    wifi::join_wifi(&mut control, WIFI_NETWORK, WIFI_PASSWORD).await;
    wifi::bring_up_stack(stack).await;
    info!("Stack is up!");

    if let Err(e) = http_client::fetch_json(stack).await {
        info!("Error fetching {}", e);
    }

    loop {
        Timer::after(Duration::from_secs(30)).await;
    }
}
