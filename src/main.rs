#![no_std]
#![no_main]
#[deny(clippy::mem_forget)]

mod devices;

use defmt::println;
use devices::{lora::LoRaRadio, types::BusSpi, types::MutexSpi};
use embassy_executor::Spawner;
use embassy_net::StackResources;
use embassy_time::{Duration, Timer};

use embedded_storage::ReadStorage;
use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::{Input, Output, Pull},
    i2c::master::I2c,
    rng::Rng,
    spi::master::Spi,
    timer::timg::TimerGroup,
};
use esp_wifi::{wifi::WifiStaDevice, EspWifiController};

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    esp_alloc::heap_allocator!(72 * 1024);
    let mut config = esp_hal::Config::default();
    config.cpu_clock = CpuClock::max();
    let peripherals = esp_hal::init(config);

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timg1.timer0);

    let mut rng = Rng::new(peripherals.RNG);
    let init = &*mk_static!(
        EspWifiController<'static>,
        esp_wifi::init(timg1.timer1, rng.clone(), peripherals.RADIO_CLK).unwrap()
    );

    let wifi = peripherals.WIFI;
    let (wifi_interface, controller) =
        match esp_wifi::wifi::new_with_mode(&init, wifi, WifiStaDevice) {
            Ok((wifi_interface, controller)) => (wifi_interface, controller),
            Err(err) => {
                esp_println::println!("[MAIN] Failed to create wifi: {:?}", err);
                loop {}
            }
        };

    let wifi_config = embassy_net::Config::dhcpv4(Default::default());
    let (stack, runner) = embassy_net::new(
        wifi_interface,
        wifi_config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        rng.random().into(),
    );

    let config = esp_hal::i2c::master::Config::default();
    let i2c0 = match I2c::new(peripherals.I2C0, config) {
        Ok(i2c) => i2c
            .with_scl(peripherals.GPIO15)
            .with_sda(peripherals.GPIO4)
            .into_async(),

        Err(err) => {
            esp_println::println!("[MAIN] Failed to create i2c: {:?}", err);
            loop {}
        }
    };
    let oled_rst = Output::new(peripherals.GPIO16, esp_hal::gpio::Level::High);

    let (tx_pin, rx_pin) = (peripherals.GPIO12, peripherals.GPIO13);
    let config = esp_hal::uart::Config::default()
        .with_baudrate(9600)
        .with_data_bits(esp_hal::uart::DataBits::_8)
        .with_parity(esp_hal::uart::Parity::None)
        .with_stop_bits(esp_hal::uart::StopBits::_1);

    let uart2 = match esp_hal::uart::Uart::new(peripherals.UART2, config) {
        Ok(uart) => uart.with_rx(rx_pin).with_tx(tx_pin).into_async(),
        Err(err) => {
            esp_println::println!("[MAIN] Failed to create uart: {:?}", err);
            loop {}
        }
    };
    let (uart2_rx, uart2_tx) = uart2.split();
    esp_println::println!("[MAIN] Uart 2 initialized");

    let mut led =
        devices::led::Led::new(Output::new(peripherals.GPIO25, esp_hal::gpio::Level::Low));
    let button = devices::button::Button::new(Input::new(peripherals.GPIO0, Pull::Up));

    led.set(devices::led::LedState::Off);
    esp_println::println!("[MAIN] Led initialized");

    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(4096);
    let config = esp_hal::spi::master::Config::default().with_frequency(fugit::HertzU32::MHz(10));
    let dma_rx_buf = match DmaRxBuf::new(rx_descriptors, rx_buffer) {
        Ok(dma) => dma,
        Err(err) => {
            esp_println::println!("[MAIN] Failed to create dma: {:?}", err);
            loop {}
        }
    };
    let dma_tx_buf = match DmaTxBuf::new(tx_descriptors, tx_buffer) {
        Ok(dma) => dma,
        Err(err) => {
            esp_println::println!("[MAIN] Failed to create dma: {:?}", err);
            loop {}
        }
    };

    let spi = match Spi::new(peripherals.SPI2, config) {
        Ok(spi) => {
            esp_println::println!("[MAIN] SPI initialized");
            spi.with_sck(peripherals.GPIO5)
                .with_miso(peripherals.GPIO19)
                .with_mosi(peripherals.GPIO27)
                .with_dma(peripherals.DMA_SPI2)
                .with_buffers(dma_rx_buf, dma_tx_buf)
                .into_async()
        }
        Err(err) => {
            esp_println::println!("[MAIN] Failed to create spi: {:?}", err);
            loop {}
        }
    };

    let lora_cs = Output::new(peripherals.GPIO18, esp_hal::gpio::Level::High);
    let lora_rst = Output::new(peripherals.GPIO14, esp_hal::gpio::Level::High);
    let lora_dio0 = Input::new(peripherals.GPIO26, Pull::Up);
    let lora_dio1 = Input::new(peripherals.GPIO35, Pull::Up);

    let flash = esp_storage::FlashStorage::new();
    println!("[MAIN] Flash capacity: {:#x}", flash.capacity());
    let spi_mutex = mk_static!(MutexSpi, MutexSpi::new(spi));
    let lora_spi = BusSpi::new(spi_mutex, lora_cs);

    let lora = match LoRaRadio::new(lora_spi, lora_rst, lora_dio0, lora_dio1, flash).await {
        Ok(lora) => lora,
        Err(err) => {
            esp_println::println!("[MAIN] Failed to create lora: {:?}", err);
            loop {}
        }
    };

    esp_hal::

    let tasks = [
        spawner.spawn(devices::gps::uart_reader(uart2_rx)),
        spawner.spawn(devices::gps::uart_writer(uart2_tx)),
        spawner.spawn(devices::wifi::network(stack, runner, controller)),
        spawner.spawn(devices::wifi::request_http(stack)),
        spawner.spawn(devices::display::display(i2c0, oled_rst)),
        spawner.spawn(devices::led::task_led(led)),
        spawner.spawn(devices::button::task_button(button)),
        // spawner.spawn(devices::lorawan::task_lorawan(lora, rng)),
        spawner.spawn(devices::lora_p2p::task_lora_p2p(lora)),
    ];

    for task in tasks.iter() {
        match task {
            Ok(_) => esp_println::println!("[MAIN] Task spawned successfully"),
            Err(e) => esp_println::println!("[MAIN] Failed to spawn task: {:?}", e),
        }
    }

    loop {
        Timer::after(Duration::from_secs(60)).await;
        esp_println::println!("[MAIN] Still alive ...");
    }
}
