use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use esp_hal::gpio::Output;

pub enum LedState {
    On,
    Off,
}

pub struct Led<'d> {
    pin: Output<'d>,
    state: LedState,
}

static LED_SIGNAL: Signal<CriticalSectionRawMutex, LedState> = Signal::new();

impl<'d> Led<'d> {
    pub fn new(pin: Output<'d>) -> Self {
        Self {
            pin,
            state: LedState::Off,
        }
    }

    pub fn on(&mut self) {
        self.pin.set_high();
    }

    pub fn off(&mut self) {
        self.pin.set_low();
    }

    pub fn set(&mut self, state: LedState) {
        self.state = state;
        match self.state {
            LedState::On => esp_println::println!("[LED] Led On"),
            LedState::Off => esp_println::println!("[LED] Led Off"),
        }
    }
}

#[embassy_executor::task]
pub async fn task_led(mut led: Led<'static>) {
    esp_println::println!("[LED] Starting LED task");
    loop {
        let timeout = Timer::after(Duration::from_millis(50));
        embassy_futures::select::select(timeout, LED_SIGNAL.wait()).await;
        if let Some(state) = LED_SIGNAL.try_take() {
            led.set(state);
        }
        match led.state {
            LedState::On => led.on(),
            LedState::Off => led.off(),
        }
    }
}
