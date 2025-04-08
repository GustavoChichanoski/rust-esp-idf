use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Input;

#[derive(Copy, Clone)]
pub enum ButtonState {
    Pressed,
    Released,
    LongPressed,
}

pub struct Button<'d> {
    pin: Input<'d>,
    state: ButtonState,
    press_duration: Duration,
}

static BUTTON_SIGNAL: Signal<CriticalSectionRawMutex, ButtonState> = Signal::new();

impl<'d> Button<'d> {
    pub fn new(pin: Input<'d>) -> Self {
        Self {
            pin: pin,
            state: ButtonState::Released,
            press_duration: Duration::from_millis(0),
        }
    }

    pub async fn update(&mut self, state: ButtonState) {
        self.state = state;
        BUTTON_SIGNAL.signal(state);
    }

    pub async fn is_pressed(&self) -> bool {
        self.pin.is_low()
    }
}

#[embassy_executor::task]
pub async fn task_button(mut button: Button<'static>) {
    esp_println::println!("[BUTTON] Starting button task");
    const DEBOUNCE_DURATION: Duration = Duration::from_millis(50);
    const LONG_PRESS_DURATION: Duration = Duration::from_millis(1000);
    let mut send = false;
    loop {
        Timer::after(DEBOUNCE_DURATION).await;
        if !button.is_pressed().await {
            if matches!(button.state, ButtonState::Released) {
                button.press_duration = Duration::from_millis(0);
                continue;
            }
            button.update(ButtonState::Released).await;
            esp_println::println!("[BUTTON] Button Released");
            continue;
        }
        if matches!(button.state, ButtonState::Released) {
            send = false;
            button.update(ButtonState::Pressed).await;
            esp_println::println!("[BUTTON] Button Pressed");
            continue;
        }
        button.press_duration += DEBOUNCE_DURATION;
        if button.press_duration > LONG_PRESS_DURATION && send == false {
            button.update(ButtonState::LongPressed).await;
            esp_println::println!("[BUTTON] Button Long Pressed");
            send = true;
        }
    }
}
