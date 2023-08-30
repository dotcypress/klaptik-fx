use mp2667::registers::{FaultFlags, SystemStatus};
use crate::*;

pub struct PowerState {
    pub overcurrent: bool,
    pub vcc: u16,
    pub status: SystemStatus,
    pub faults: FaultFlags,
}

pub struct PowerController {
    adc: Adc,
    charger: Charger,
    power_en: PowerEn,
    power_fault: PowerFault,
    sense: VccSense,
}

impl PowerController {
    pub fn new(
        adc: Adc,
        sense: VccSense,
        charger: Charger,
        power_en: PowerEn,
        power_fault: PowerFault,
    ) -> Self {
        Self {
            charger,
            adc,
            sense,
            power_en,
            power_fault,
        }
    }

    pub fn power_on(&mut self) {
        self.power_en.set_high().ok();
    }

    pub fn power_off(&mut self) {
        self.power_en.set_low().ok();
    }

    pub fn state(&mut self) -> Result<PowerState, hal::i2c::Error> {
        let overcurrent = self.power_fault.is_low().unwrap_or_default();
        let vcc = self.adc.read(&mut self.sense).unwrap_or_default();
        let status = self.charger.get_status()?;
        let faults = self.charger.get_faults()?;
        Ok(PowerState {
            overcurrent,
            vcc,
            status,
            faults,
        })
    }
}
