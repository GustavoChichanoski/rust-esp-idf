use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, pipe::Pipe};
use esp_hal::{
    uart::{UartRx, UartTx},
    Async,
};
use nmea0183::{ParseResult, Parser};

static DATAPIPE_UART: Pipe<CriticalSectionRawMutex, UART_BUF_SIZE> = Pipe::new();
const UART_BUF_SIZE: usize = 4048;

#[embassy_executor::task]
pub async fn uart_writer(mut tx: UartTx<'static, Async>) {
    esp_println::println!("[GPS] UART TX initialized");
    use core::fmt::Write;

    // Mensagem inicial
    match embedded_io_async::Write::write(
        &mut tx,
        b"UART initialized. Enter text followed by CTRL-D.\r\n",
    )
    .await
    {
        Err(e) => esp_println::println!("[GPS] Tx Error: {:?}", e),
        Ok(len) => esp_println::println!("[GPS] Wrote {} bytes", len),
    }
    embedded_io_async::Write::flush(&mut tx).await.unwrap();

    let mut buffer = [0u8; UART_BUF_SIZE];

    loop {
        // Lê os dados disponíveis do Pipe
        let len = DATAPIPE_UART.read(&mut buffer).await;

        // Envia os dados recebidos para a UART
        match write!(
            &mut tx,
            "Received {} bytes: {}\r\n",
            len,
            core::str::from_utf8(&buffer[..len]).unwrap_or("[Invalid UTF-8]")
        ) {
            Err(e) => esp_println::println!("[GPS] Tx Error: {:?}", e),
            Ok(len) => len,
        };
        embedded_io_async::Write::flush(&mut tx).await.unwrap();
    }
}

fn handle_parsed_result(result: Result<ParseResult, &str>) {
    match result {
        Ok(ParseResult::GGA(Some(gga))) => {
            esp_println::println!(
                "[GPS] GGA: Time: {:?}, Lat: {:?}, Lon: {:?}, Alt: {:?}",
                gga.time,
                gga.latitude,
                gga.longitude,
                gga.altitude
            );
        }
        Ok(_) => {}
        Err(_) => {}
    }
}

#[embassy_executor::task]
pub async fn uart_reader(mut rx: UartRx<'static, Async>) {
    esp_println::println!("[GPS] UART RX initialized");
    let mut rx_buf = [0u8; UART_BUF_SIZE];
    let mut parser = Parser::new();
    loop {
        match embedded_io_async::Read::read(&mut rx, &mut rx_buf).await {
            Err(e) => esp_println::println!("[GPS] Rx Error: {:?}", e),
            Ok(len) => {
                if len == 0 {
                    continue;
                }
                for b in &rx_buf[..len] {
                    if let Some(result) = parser.parse_from_byte(*b) {
                        handle_parsed_result(result);
                    }
                }
                rx_buf.fill_with(Default::default);
            }
        }
    }
}
