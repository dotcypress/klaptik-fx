use hal::gpio::*;
use hal::prelude::*;
use hal::rcc::Rcc;
use hal::stm32;
use hal::timer::*;
use klaptik::drivers::st7567;
use shared_bus_rtic::SharedBus;

pub type I2cDev = hal::i2c::I2c<stm32::I2C2, I2cSda, I2cClk>;
pub type SpiDev = hal::spi::Spi<stm32::SPI2, (SpiClk, SpiMiso, SpiMosi)>;

pub type Backlight = pwm::PwmPin<stm32::TIM14, Channel1>;
pub type DisplayDelay = delay::Delay<stm32::TIM1>;
pub type DisplayDriver = st7567::ST7567<SharedBus<SpiDev>, LcdReset, LcdCS, LcdDC>;

pub type Charger = mp2667::MP2667<SharedBus<I2cDev>>;

// Qwiic I2C
pub type I2cClk = PA11<Output<OpenDrain>>;
pub type I2cSda = PA12<Output<OpenDrain>>;

// SPI
pub type SpiClk = PB8<DefaultMode>;
pub type SpiMiso = PB6<DefaultMode>;
pub type SpiMosi = PB7<DefaultMode>;

// Display
pub type LcdDC = PC6<Output<PushPull>>;
pub type LcdCS = PA15<Output<PushPull>>;
pub type LcdReset = PA8<Output<PushPull>>;
pub type LcdBacklight = PB1<DefaultMode>;

// EEPROM
pub type EepromCS = PB3<Output<PushPull>>;
pub type EepromWP = PC15<Output<PushPull>>;

// Power
pub type PowerEn = PB4<Output<PushPull>>;
pub type PowerInt = PB5<Input<Floating>>;
pub type PowerFault = PC14<Input<Floating>>;
pub type VccSense = PB0<DefaultMode>;

// GPIO
pub type Gpio0 = PA0<Input<Floating>>;
pub type Gpio1 = PA1<Input<Floating>>;
pub type Gpio2 = PA2<Input<Floating>>;
pub type Gpio3 = PA3<Input<Floating>>;
pub type Gpio4 = PA4<Input<Floating>>;
pub type Gpio5 = PA5<Input<Floating>>;
pub type Gpio6 = PA6<Input<Floating>>;
pub type Gpio7 = PA7<Input<Floating>>;

// SWD
pub type SwdIo = PA13<DefaultMode>;
pub type SwdClk = PA14<DefaultMode>;

pub struct Gpio {
    pub gpio0: Gpio0,
    pub gpio1: Gpio1,
    pub gpio2: Gpio2,
    pub gpio3: Gpio3,
    pub gpio4: Gpio4,
    pub gpio5: Gpio5,
    pub gpio6: Gpio6,
    pub gpio7: Gpio7,
}

pub struct Pins {
    // Qwiic I2C
    pub i2c_clk: I2cClk,
    pub i2c_sda: I2cSda,

    // SPI
    pub spi_clk: SpiClk,
    pub spi_miso: SpiMiso,
    pub spi_mosi: SpiMosi,

    // Display
    pub lcd_dc: LcdDC,
    pub lcd_backlight: LcdBacklight,
    pub lcd_cs: LcdCS,
    pub lcd_reset: LcdReset,

    // EEPROM
    pub eeprom_cs: EepromCS,
    pub eeprom_wp: EepromWP,

    // Power
    pub power_en: PowerEn,
    pub power_int: PowerInt,
    pub power_fault: PowerFault,
    pub vcc_sense: VccSense,

    // GPIO
    pub gpio: Gpio,

    // SWD
    pub swd_io: SwdIo,
    pub swd_clk: SwdClk,
}

impl Pins {
    pub fn new(
        gpioa: stm32::GPIOA,
        gpiob: stm32::GPIOB,
        gpioc: stm32::GPIOC,
        rcc: &mut Rcc,
    ) -> Self {
        let port_a = gpioa.split(rcc);
        let port_b = gpiob.split(rcc);
        let port_c = gpioc.split(rcc);

        Self {
            // SWD
            swd_io: port_a.pa13,
            swd_clk: port_a.pa14,

            // Qwiic I2C
            i2c_clk: port_a
                .pa11
                .set_speed(Speed::High)
                .into_open_drain_output_in_state(PinState::High),
            i2c_sda: port_a
                .pa12
                .set_speed(Speed::High)
                .into_open_drain_output_in_state(PinState::High),

            //SPI
            spi_clk: port_b.pb8.set_speed(Speed::VeryHigh),
            spi_miso: port_b.pb6.set_speed(Speed::VeryHigh),
            spi_mosi: port_b.pb7.set_speed(Speed::VeryHigh),

            // Display
            lcd_cs: port_a.pa15.into_push_pull_output_in_state(PinState::High),
            lcd_dc: port_c.pc6.into(),
            lcd_reset: port_a.pa8.into(),
            lcd_backlight: port_b.pb1,

            // EEPROM
            eeprom_cs: port_b.pb3.into_push_pull_output_in_state(PinState::High),
            eeprom_wp: port_c.pc15.into_push_pull_output_in_state(PinState::High),

            power_en: port_b.pb4.into(),
            power_int: port_b.pb5.into(),
            power_fault: port_c.pc14.into(),
            vcc_sense: port_b.pb0,

            // GPIO
            gpio: Gpio {
                gpio0: port_a.pa0.into(),
                gpio1: port_a.pa1.into(),
                gpio2: port_a.pa2.into(),
                gpio3: port_a.pa3.into(),
                gpio4: port_a.pa4.into(),
                gpio5: port_a.pa5.into(),
                gpio6: port_a.pa6.into(),
                gpio7: port_a.pa7.into(),
            },
        }
    }
}
