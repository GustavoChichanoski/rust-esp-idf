use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X9, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::{Point, Primitive, Size},
    primitives::{PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Baseline, Text},
    Drawable,
};
use esp_hal::{gpio::Output, i2c::master::I2c, Async};
use heapless::String;
use qrcodegen_no_heap::{QrCode, QrCodeEcc, Version};
use ssd1306::{mode::DisplayConfigAsync, size::DisplaySize128x64, I2CDisplayInterface};

pub static DISPLAY_SIGNAL: Signal<CriticalSectionRawMutex, String<64>> = Signal::new();

#[derive(Debug)]
enum DisplayError {
    DisplayError,
    QrError,
}

type Display<'a> = ssd1306::Ssd1306Async<
    ssd1306::prelude::I2CInterface<I2c<'a, Async>>,
    DisplaySize128x64,
    ssd1306::mode::BufferedGraphicsModeAsync<DisplaySize128x64>,
>;


fn show_qr(
    display: &mut Display,
    qr: &QrCode,
    qr_style: PrimitiveStyle<BinaryColor>,
) -> Result<(), DisplayError> {
    let qr_size = qr.size();
    let scale = 2;
    let offset_x = 5;
    let offset_y = 0;
    for y in 0..qr_size {
        for x in 0..qr_size {
            if qr.get_module(x, y) {
                continue;
            }
            match Rectangle::new(
                Point::new(offset_x + x * scale, offset_y + y * scale),
                Size::new(scale as u32, scale as u32),
            )
            .into_styled(qr_style)
            .draw(display)
            {
                Ok(_) => (),
                Err(e) => {
                    esp_println::print!("[OLED] Draw failed: {:?}", e);
                    return Err(DisplayError::QrError);
                }
            };
        }
    }
    Ok(())
}

async fn show_table<'a>(
    display: &mut Display<'a>,
    text_style: MonoTextStyleBuilder<'a, BinaryColor>,
    freq: &str,
    snr: &str,
    rssi: &str,
) {
    const START_X: i32 = 50;
    const START_Y: i32 = 5;
    const LINE_SPACING: i32 = 12;

    let labels = ["MHz:", "SNR:", "RSSI:"];
    let values = [freq, snr, rssi];

    // 1. Create a single reusable String buffer
    let mut msg = heapless::String::<64>::new();

    for (i, (label, value)) in labels.iter().zip(values.iter()).enumerate() {
        // 2. Clear the buffer for each line
        msg.clear();

        // 3. Improved error handling
        if let Err(e) = core::fmt::write(&mut msg, core::format_args!("{} {}", label, value)) {
            esp_println::println!("[OLED] Format error (line {}): {:?}", i, e);
            continue;
        }

        // 4. Calculate Y position safely
        let y_pos = START_Y + (i as i32 * LINE_SPACING);

        // 5. Draw with explicit error reporting
        match Text::with_baseline(
            &msg,
            Point::new(START_X, y_pos),
            text_style.build(),
            Baseline::Top,
        )
        .draw(display)
        {
            Ok(_) => (),
            Err(e) => esp_println::println!("[OLED] Draw failed (line {}): {:?}", i, e),
        }
    }
}

#[embassy_executor::task]
pub async fn display(i2c: I2c<'static, Async>, mut reset: Output<'static>) {
    esp_println::println!("[OLED] Starting display task");
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = ssd1306::Ssd1306Async::new(
        interface,
        DisplaySize128x64,
        ssd1306::prelude::DisplayRotation::Rotate0,
    )
    .into_buffered_graphics_mode();

    display
        .reset(&mut reset, &mut embassy_time::Delay)
        .await
        .unwrap();

    if let Err(e) = display.init().await {
        esp_println::println!("[OLED] Display init error: {:#?}", e);
        return;
    }

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X9)
        .text_color(BinaryColor::On);

    let qr_style = PrimitiveStyleBuilder::new()
        .fill_color(BinaryColor::On)
        .build();

    let mut out_buffer = [0u8; Version::MAX.buffer_len()];
    let mut temp_buffer = [0u8; Version::MAX.buffer_len()];

    let freq = "868.1";
    let snr = "10 dB";
    let rssi = "-98 dBm";
    loop {
        match DISPLAY_SIGNAL.try_take() {
            Some(value) => {
                match generate_qr_code(value, &mut temp_buffer, &mut out_buffer) {
                    Some(qr) => match show_qr(&mut display, &qr, qr_style) {
                        Ok(()) => (),
                        Err(e) => esp_println::println!("[OLED] QR error: {:#?}", e),
                    },
                    None => (),
                };
            }
            None => (),
        };

        show_table(&mut display, text_style, freq, snr, rssi).await;
        match display.flush().await {
            Ok(()) => (),
            // Err(e) => esp_println::println!("[OLED] Display flush error: {:#?}", e),
            Err(_) => (),
        }
    }
}

fn generate_qr_code<'a>(
    value: String<64>,
    temp: &'a mut [u8],
    out: &'a mut [u8],
) -> Option<QrCode<'a>> {
    let qr = match QrCode::encode_text(
        value.as_str(),
        temp,
        out,
        QrCodeEcc::Low,
        Version::MIN,
        Version::MAX,
        None,
        true,
    ) {
        Ok(qr) => qr,
        Err(_) => return None,
    };
    return Some(qr);
}
