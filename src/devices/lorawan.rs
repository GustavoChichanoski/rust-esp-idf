use embassy_time::{Duration, Timer};
use esp_hal::rng::Rng;
use lora_phy::lorawan_radio::LorawanRadio;
use lorawan_device::{
    async_device::{Device, EmbassyTimer},
    default_crypto::DefaultFactory as Crypto,
    region, AppEui, AppKey, AppSKey, DevAddr, DevEui, NwkSKey,
};

use super::lora::LoRaRadio;
const _MAX_TX_POWER: u8 = 20;

// OTAA Credentials
const _DEFAULT_DEVEUI: [u8; 8] = [0x4d, 0x89, 0x64, 0x17, 0xe7, 0x49, 0x2c, 0xbb];
const _DEFAULT_APPEUI: [u8; 8] = [0x82, 0xc5, 0x4b, 0x04, 0x0d, 0x29, 0x70, 0xa9];
const _DEFAULT_APPKEY: [u8; 16] = [
    0x72, 0xad, 0x47, 0x7e, 0xca, 0xe4, 0xa7, 0x80, 0xb5, 0xae, 0x93, 0xbf, 0xac, 0x7a, 0x04, 0xbb,
];

// ABP Credentials
const _DEFAULT_DEVADDR: [u8; 4] = [0xD2, 0xFC, 0x8B, 0xC8];
const _DEFAULT_NWKSKEY: [u8; 16] = [
    0x17, 0x2A, 0xE2, 0xDC, 0xD3, 0xA3, 0xC6, 0xE2, 0x39, 0xE4, 0xDC, 0x73, 0x00, 0x23, 0xF0, 0x7E,
];
const _DEFAULT_APPSKEY: [u8; 16] = [
    0xAA, 0x70, 0x08, 0x19, 0xFE, 0x52, 0x8C, 0x91, 0x6B, 0xEF, 0x1D, 0xDE, 0x04, 0x55, 0x1F, 0x95,
];

#[embassy_executor::task]
pub async fn task_lorawan(mut lora: LoRaRadio<'static>, rng: Rng) {
    esp_println::println!("[LoRa WAN] Activating LoRaWAN network using OTAA ...");
    lora.radio.init().await.unwrap();
    lora.device_nonce = u16::from(lora.device_nonce).wrapping_add(1);
    // Convert the P2P radio into a LoRaWAN radio
    let radio: LorawanRadio<_, _, _MAX_TX_POWER> = lora.radio.into();
    let data = [0xAB, 0xCD, 0xEF];
    let mut au915 = region::AU915::new();
    au915.set_join_bias(region::Subband::_1);
    // Create the LoRaWAN device
    let mut device: Device<_, Crypto, _, _> =
        Device::new(au915.into(), radio, EmbassyTimer::new(), rng);

    esp_println::println!("[LoRa WAN] Activating device...");

    // Create the ABP join mode with the credentials.
    let join_mode = lorawan_device::JoinMode::OTAA {
        appeui: AppEui::from(_DEFAULT_APPEUI),
        appkey: AppKey::from(_DEFAULT_APPKEY),
        deveui: DevEui::from(_DEFAULT_DEVEUI),
    };
    

    // let join_mode = lorawan_device::JoinMode::ABP {
    //     devaddr: DevAddr::from(_DEFAULT_DEVADDR),
    //     nwkskey: NwkSKey::from(_DEFAULT_NWKSKEY),
    //     appskey: AppSKey::from(_DEFAULT_APPSKEY),
    // };

    loop {
        // In ABP, join() will not perform an over-the-air join but will instead configure the device.
        match device.join(&join_mode).await {
            Ok(join_response) => match join_response {
                lorawan_device::async_device::JoinResponse::JoinSuccess => {
                    esp_println::println!("[LoRa WAN] Joined network.");
                    break;
                }
                lorawan_device::async_device::JoinResponse::NoJoinAccept => {
                    esp_println::println!("[LoRa WAN] Rejoined network.");
                }
            },
            Err(err) => {
                esp_println::println!("[LoRa WAN] OTAA activation failed: {:?}", err);
                return;
            }
        }
        Timer::after(Duration::from_millis(1000)).await;
    }

    // Now send uplink messages in a loop.
    loop {
        esp_println::println!("[LoRa WAN] Sending uplink to Everynet...");
        // The payload here is a sample byte array; replace with your application data.
        match device.send(&data, 2, false).await {
            Ok(lorawan_device::async_device::SendResponse::DownlinkReceived(_)) => {
                esp_println::println!("[LoRa WAN] Everynet downlink received!");
                while let Some(downlink) = device.take_downlink() {
                    esp_println::println!("[LoRa WAN] Downlink Data: {:?}", downlink.data);
                }
                break;
            }
            Ok(_) => {
                esp_println::println!("[LoRa WAN] Uplink sent successfully.");
            }
            Err(err) => {
                esp_println::println!("[LoRa WAN] Failed to send uplink: {:?}", err);
            }
        }
        // Wait 10 seconds before sending the next uplink.
        embassy_time::Timer::after(Duration::from_secs(10)).await;
    }
}
