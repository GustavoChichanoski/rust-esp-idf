use embassy_futures::select::{select, Either};
use embassy_time::{with_timeout, Duration};
use embedded_hal::digital::OutputPin;
use embedded_hal_async::digital::Wait;
use lora_phy::{mod_params::RadioError, mod_traits::InterfaceVariant};

/// SX1276 Interface with RX/TX switching
pub struct InterfaceSx1276<CTRL, WAIT> {
    dio0: WAIT,
    dio1: WAIT,
    reset: CTRL,
    rf_switch_rx: Option<CTRL>,
    rf_switch_tx: Option<CTRL>,
}

impl<CTRL, WAIT> InterfaceSx1276<CTRL, WAIT>
where
    CTRL: OutputPin,
    WAIT: Wait,
{
    /// Create a new SX1276 interface
    pub fn new(
        dio0: WAIT,
        dio1: WAIT,
        reset: CTRL,
        rf_switch_rx: Option<CTRL>,
        rf_switch_tx: Option<CTRL>,
    ) -> Result<Self, RadioError> {
        Ok(Self {
            dio0,
            dio1,
            reset,
            rf_switch_rx,
            rf_switch_tx,
        })
    }
}

impl<CTRL, WAIT> InterfaceVariant for InterfaceSx1276<CTRL, WAIT>
where
    WAIT: Wait,
    CTRL: OutputPin,
{
    /// Wait for an interrupt from DIO0 or DIO1
    async fn await_irq(&mut self) -> Result<(), RadioError> {
        match with_timeout(
            Duration::from_millis(100),
            select(self.dio0.wait_for_high(), self.dio1.wait_for_high()),
        )
        .await
        {
            Ok(Either::First(value)) => {
                defmt::info!("DIO0 interrupt triggered");
                match value {
                    Ok(()) => Ok(()),
                    Err(_) => Err(RadioError::Irq),
                }
            }
            Ok(Either::Second(value)) => {
                defmt::info!("DIO1 interrupt triggered");
                match value {
                    Ok(()) => Ok(()),
                    Err(_) => Err(RadioError::Irq),
                }
            }
            Err(_) => Ok(()),
        }
    }

    /// Reset the SX1276 chip
    async fn reset(&mut self, delay: &mut impl lora_phy::DelayNs) -> Result<(), RadioError> {
        self.reset.set_low().map_err(|_| RadioError::Reset)?;
        delay.delay_ms(10).await;
        self.reset.set_high().map_err(|_| RadioError::Reset)?;
        delay.delay_ms(10).await;
        Ok(())
    }

    /// Wait until the SX1276 is no longer busy
    async fn wait_on_busy(&mut self) -> Result<(), RadioError> {
        // SX1276 does not have a dedicated busy pin, so return Ok.
        Ok(())
    }

    /// Enable RX mode by setting the RF switch
    async fn enable_rf_switch_rx(&mut self) -> Result<(), RadioError> {
        if let Some(tx_pin) = &mut self.rf_switch_tx {
            tx_pin.set_low().map_err(|_| RadioError::RfSwitchTx)?;
        }
        if let Some(rx_pin) = &mut self.rf_switch_rx {
            rx_pin.set_high().map_err(|_| RadioError::RfSwitchRx)?;
        }
        Ok(())
    }

    /// Enable TX mode by setting the RF switch
    async fn enable_rf_switch_tx(&mut self) -> Result<(), RadioError> {
        if let Some(rx_pin) = &mut self.rf_switch_rx {
            rx_pin.set_low().map_err(|_| RadioError::RfSwitchRx)?;
        }
        if let Some(tx_pin) = &mut self.rf_switch_tx {
            tx_pin.set_high().map_err(|_| RadioError::RfSwitchTx)?;
        }
        Ok(())
    }

    /// Disable RF switches (low power mode)
    async fn disable_rf_switch(&mut self) -> Result<(), RadioError> {
        if let Some(rx_pin) = &mut self.rf_switch_rx {
            rx_pin.set_low().map_err(|_| RadioError::RfSwitchRx)?;
        }
        if let Some(tx_pin) = &mut self.rf_switch_tx {
            tx_pin.set_low().map_err(|_| RadioError::RfSwitchTx)?;
        }
        Ok(())
    }
}
