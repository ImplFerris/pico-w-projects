use embassy_executor::Spawner;

use embassy_rp::Peri;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::Pio;

use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};

use static_cell::StaticCell;

// defmt Logging
use defmt::{info, unwrap};

use embassy_net::{Config, StackResources};

use embassy_rp::dma;

use crate::Irqs;

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<
        'static,
        cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>,
        // cyw43::Cyw43439,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

pub async fn init_cyw43(
    spawner: Spawner,
    pio0: Peri<'static, PIO0>,
    dma_ch0: Peri<'static, DMA_CH0>,
    pin23: Peri<'static, embassy_rp::peripherals::PIN_23>,
    pin24: Peri<'static, embassy_rp::peripherals::PIN_24>,
    pin25: Peri<'static, embassy_rp::peripherals::PIN_25>,
    pin29: Peri<'static, embassy_rp::peripherals::PIN_29>,
) -> (cyw43::NetDriver<'static>, cyw43::Control<'static>) {
    let fw = cyw43::aligned_bytes!("../../../firmware/43439A0.bin");
    let clm = cyw43::aligned_bytes!("../../../firmware/43439A0_clm.bin");
    let nvram = cyw43::aligned_bytes!("../../../firmware/nvram_rp2040.bin");

    let pwr = Output::new(pin23, Level::Low);
    let cs = Output::new(pin25, Level::High);
    let mut pio = Pio::new(pio0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        pin24,
        pin29,
        dma::Channel::new(dma_ch0, Irqs),
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;

    spawner.spawn(unwrap!(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    (net_device, control)
}

pub fn init_net_stack(
    spawner: Spawner,
    net_device: cyw43::NetDriver<'static>,
    seed: u64,
) -> embassy_net::Stack<'static> {
    let config = Config::dhcpv4(Default::default());
    // Use static IP configuration instead of DHCP
    // let net_config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
    //     address: embassy_net::Ipv4Cidr::new(embassy_net::Ipv4Address::new(192, 168, 69, 2), 24),
    //     dns_servers: heapless::Vec::new(),
    //     gateway: Some(embassy_net::Ipv4Address::new(192, 168, 69, 1)),
    // });

    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let (stack, net_runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    spawner.spawn(unwrap!(net_task(net_runner)));

    stack
}

/// Join the Wi-Fi network, retrying until it succeeds.
pub async fn join_wifi(control: &mut cyw43::Control<'static>, ssid: &str, password: &str) {
    while let Err(err) = control
        .join(ssid, cyw43::JoinOptions::new(password.as_bytes()))
        .await
    {
        info!("join failed: {:?}", err);
    }
}

/// Wait for link + DHCP.
pub async fn bring_up_stack(stack: embassy_net::Stack<'static>) {
    info!("waiting for link...");
    stack.wait_link_up().await;

    info!("waiting for DHCP...");
    stack.wait_config_up().await;

    if let Some(cfg) = stack.config_v4() {
        info!("IP: {:?}", cfg.address);
        info!("Gateway: {:?}", cfg.gateway);
        info!("DNS: {:?}", cfg.dns_servers);
    }

    // Custom DNS Servers
    if let Some(mut cfg) = stack.config_v4() {
        cfg.dns_servers = heapless::Vec::from_slice(&[
            embassy_net::Ipv4Address::new(1, 1, 1, 1),
            embassy_net::Ipv4Address::new(8, 8, 8, 8),
        ])
        .expect("DNS server list exceeds heapless::Vec capacity");

        stack.set_config_v4(embassy_net::ConfigV4::Static(cfg));
        info!("Overrode DNS servers, keeping DHCP address/gateway");
    }
}
