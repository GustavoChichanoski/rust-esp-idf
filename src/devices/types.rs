use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use esp_hal::{gpio::Output, spi::master::SpiDmaBus};

pub type LoRaSpi = SpiDmaBus<'static, esp_hal::Async>;
pub type BusSpi<'a> = embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice<
    'a,
    CriticalSectionRawMutex,
    LoRaSpi,
    Output<'a>,
>;

pub type MutexSpi<'a> = Mutex<CriticalSectionRawMutex, LoRaSpi>;
