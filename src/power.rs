use crate::*;
use mp2667::registers::{ChargeStatus, FaultFlags, SystemStatus};

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
        let mut cfg = self.charger.get_charge_current_control().unwrap();
        cfg.set_charge_current(0b01111);
        self.charger.set_charge_current_control(cfg).unwrap();

        defmt::info!("vcc: {} | overcurrent: {}", vcc, overcurrent,);
        defmt::info!("status: {}", defmt::Debug2Format(&status));
        defmt::info!("faults: {}", defmt::Debug2Format(&faults));

        match status.charge_status() {
            ChargeStatus::NotCharging => defmt::info!("NotCharging"),
            ChargeStatus::PreCharge => defmt::info!("PreCharge"),
            ChargeStatus::Charge => defmt::info!("Charge"),
            ChargeStatus::ChargeDone => defmt::info!("ChargeDone"),
        }

        Ok(PowerState {
            overcurrent,
            vcc,
            status,
            faults,
        })
    }
}
