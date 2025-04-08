use crate::devices::lora::LoRaRadio;
use embassy_time::{Duration, Timer};
use lora_phy::{
    mod_params::{Bandwidth, CodingRate, ModulationParams, PacketParams, SpreadingFactor},
    RxMode,
};

const PREAMBLE_LENGTH: u16 = 12;

#[derive(Debug)]
pub enum P2PErrors {
    PrepareForTx,
    Tx,
    PrepareForRx,
    Rx,
}

impl core::fmt::Display for P2PErrors {
    /// Format the error as a string, suitable for display to the user.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            P2PErrors::PrepareForTx => write!(f, "Failed to prepare for transmission"),
            P2PErrors::Tx => write!(f, "Transmission failed"),
            P2PErrors::PrepareForRx => write!(f, "Failed to prepare for reception"),
            P2PErrors::Rx => write!(f, "Reception failed"),
        }
    }
}

/// Starts a loop that sends and receives LoRa P2P messages with the given LoRa radio.
///
/// The function will first create modulation and packet parameters for transmission and
/// reception. It will then enter a loop that sleeps for 6 seconds, receives a message,
/// sleeps for 1 second, sends a message, sleeps for 1 second, and then repeats.
///
/// If any step fails, an error message will be printed to the console and the function will
/// exit.
#[embassy_executor::task]
pub async fn task_lora_p2p(mut lora: LoRaRadio<'static>) {
    esp_println::println!("[LoRa] Starting LoRa P2P ...");
    let frequency: u32 = 904_000_000;
    let modulation = match lora.radio.create_modulation_params(
        SpreadingFactor::_11,
        Bandwidth::_500KHz,
        CodingRate::_4_5,
        frequency,
    ) {
        Ok(params) => params,
        Err(err) => {
            esp_println::println!("[LoRa P2P] Failed to create modulation params: {:?}", err);
            return;
        }
    };

    let mut tx_params =
        match lora
            .radio
            .create_tx_packet_params(PREAMBLE_LENGTH, false, true, false, &modulation)
        {
            Ok(params) => params,
            Err(err) => {
                esp_println::println!("[LoRa P2P] Failed to create tx packet params: {:?}", err);
                return;
            }
        };

    let mut rx = [0u8; 255];
    let mut rx_params = match lora.radio.create_rx_packet_params(
        PREAMBLE_LENGTH,
        false,
        rx.len() as u8,
        true,
        false,
        &modulation,
    ) {
        Ok(params) => params,
        Err(err) => {
            esp_println::println!("[LoRa P2P] Failed to create rx packet params: {:?}", err);
            return;
        }
    };

    loop {
        match p2p_tx_msg(&mut lora, &mut tx_params, &modulation).await {
            Ok(()) => {}
            Err(err) => {
                esp_println::println!("[LoRa P2P] Failed to send message: {:?}", err);
                return;
            }
        }
        Timer::after(Duration::from_secs(1)).await;
        rx = [0u8; 255];
        match p2p_rx_msg(&mut lora, &mut rx, &mut rx_params, &modulation).await {
            Ok(rx_len) => {
                // Remove this erroneous line from your original code:
                for i in 0..rx_len {
                    esp_println::print!("0x{:02X} ", rx[i as usize]);
                }
                esp_println::print!("\n");
            }
            Err(err) => {
                esp_println::println!("[LoRa P2P] Failed to receive message before tx: {:?}", err);
                continue;
            }
        }
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// Sends a message over LoRa in P2P mode.
///
/// The method will first prepare the radio for transmission and then send the message.
/// If any step fails, an error will be returned.
///
/// # Errors
///
/// The method will return a `P2PErrors` if any step fails:
///
/// * `PrepareForTx`: if preparing for transmission fails
/// * `Tx`: if sending the message fails
///
async fn p2p_tx_msg(
    lora: &mut LoRaRadio<'static>,
    tx_params: &mut PacketParams,
    modulation: &ModulationParams,
) -> Result<(), P2PErrors> {
    esp_println::println!("[LoRa P2P] Sending...");

    let mut tx: [u8; 52] = [0; 52];
    for i in 0..tx.len() {
        tx[i as usize] = i as u8;
    }

    // Add timeout for prepare_for_tx
    match lora
        .radio
        .prepare_for_tx(&modulation, tx_params, 20, &tx)
        .await
    {
        Ok(()) => esp_println::println!("[LoRa P2P] Prepared for tx"),
        Err(err) => {
            esp_println::println!("[LoRa P2P] Failed to prepare for tx: {:?}", err);
            return Err(P2PErrors::PrepareForTx);
        }
    }
    // Add timeout for tx()
    match lora.radio.tx().await {
        Ok(()) => esp_println::println!("[LoRa P2P] Message sent"),
        Err(err) => {
            esp_println::println!("[LoRa P2P] Failed to send message: {:?}", err);
            return Err(P2PErrors::Tx);
        }
    }
    Ok(())
}

/// Receives a message over LoRa in P2P mode.
///
/// The method prepares the radio for receiving data and then attempts to receive a message.
/// The received message length and packet status are returned upon success. If any step fails,
/// an error is returned.
///
/// # Arguments
///
/// * `lora` - A mutable reference to the LoRa radio.
/// * `rx` - A mutable byte slice to store the received message.
/// * `rx_params` - Transmission packet parameters.
/// * `modulation` - Modulation parameters for the radio.
///
/// # Returns
///
/// * `Ok(u8)` - The length of the received message if successful.
/// * `Err(P2PErrors)` - An error if the preparation for receiving or the reception itself fails.
///
/// # Errors
///
/// The method returns a `P2PErrors` if any step fails:
///
/// * `PrepareForRx` - If preparing for reception fails.
/// * `Rx` - If receiving the message fails.
async fn p2p_rx_msg(
    lora: &mut LoRaRadio<'static>,
    rx: &mut [u8],
    rx_params: &mut PacketParams,
    modulation: &ModulationParams,
) -> Result<u8, P2PErrors> {
    esp_println::println!("[LoRa P2P] Receiving...");

    match lora
        .radio
        .prepare_for_rx(RxMode::Single(512), &modulation, &rx_params)
        .await
    {
        Ok(()) => esp_println::println!("[LoRa P2P] Prepared for rx"),
        Err(err) => {
            esp_println::println!("[LoRa P2P] Failed to prepare for rx: {:?}", err);
            return Err(P2PErrors::PrepareForRx);
        }
    }

    let (rx_len, status) = match lora.radio.rx(&rx_params, rx).await {
        Ok((rx_len, status)) => (rx_len, status),
        Err(err) => {
            esp_println::println!("[LoRa P2P] Failed to receive: {:?}", err);
            return Err(P2PErrors::PrepareForRx);
        }
    };

    esp_println::println!("[LoRa P2P] Received: {:#?}", rx_len);
    esp_println::print!(
        "[LoRa P2P] rssi: {:#?} | snr: {:#?} | payload: ",
        status.rssi,
        status.snr
    );

    if status.rssi < 50 {
        return Ok(rx_len);
    }

    for i in 0..rx_len {
        esp_println::print!("0x{:02X} ", rx[i as usize]);
    }
    esp_println::print!("\n");
    let mut msg = heapless::String::<1024>::new();
    core::fmt::write(
        &mut msg,
        core::format_args!("rssi: {} snr: {}", status.rssi, status.snr),
    )
    .map_err(|_| P2PErrors::Rx)?;
    return Ok(rx_len);
}
