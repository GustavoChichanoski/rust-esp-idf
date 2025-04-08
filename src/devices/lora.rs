use embedded_storage::Storage;
use esp_hal::gpio::{Input, Output};
use lora_phy::{
    sx127x::{Sx1276, Sx127x},
    LoRa,
};

use super::{iv::InterfaceSx1276, types::BusSpi};

#[derive(Debug)]
pub enum LoraTaskError {
    InitFailed,
}

pub struct LoRaRadio<'d> {
    pub radio: LoRa<
        Sx127x<BusSpi<'d>, InterfaceSx1276<Output<'d>, Input<'d>>, Sx1276>,
        embassy_time::Delay,
    >,
    pub device_nonce: u16,
    pub device_addr: [u8; 4],
    pub device_eui: [u8; 8],
    pub application_eui: [u8; 8],
    pub application_session_key: [u8; 16],
    pub network_session_key: [u8; 16],
    pub storage: esp_storage::FlashStorage,
}

impl<'d> LoRaRadio<'d> {
    pub async fn new(
        spi: BusSpi<'d>,
        reset: Output<'d>,
        dio0: Input<'d>,
        dio1: Input<'d>,
        storage: esp_storage::FlashStorage,
    ) -> Result<Self, LoraTaskError> {
        let config = lora_phy::sx127x::Config {
            chip: Sx1276,
            tcxo_used: true,
            tx_boost: false,
            rx_boost: false,
        };
        let iv = InterfaceSx1276::new(dio0, dio1, reset, None, None)
            .map_err(|_| LoraTaskError::InitFailed)?;
        let driver_lora = Sx127x::new(spi, iv, config);
        let lora = LoRa::new(driver_lora, true, embassy_time::Delay)
            .await
            .map_err(|_| LoraTaskError::InitFailed)?;

        Ok(Self {
            radio: lora,
            device_nonce: 0,
            device_addr: [0; 4],
            device_eui: [0; 8],
            application_eui: [0; 8],
            application_session_key: [0; 16],
            network_session_key: [0; 16],
            storage,
        })
    }

    pub fn save(&mut self) {
        let mut buffer = [0; 40];
        buffer[..4].copy_from_slice(&self.device_addr);
        buffer[4..12].copy_from_slice(&self.device_eui);
        buffer[12..20].copy_from_slice(&self.application_eui);
        buffer[20..28].copy_from_slice(&self.application_session_key);
        buffer[28..36].copy_from_slice(&self.network_session_key);
        buffer[36..40].copy_from_slice(&self.device_nonce.to_le_bytes());
        match self.storage.write(0, &buffer) {
            Ok(()) => (),
            Err(e) => esp_println::println!("[LoRa] Error writing to flash: {:#?}", e),
        };
    }
}
