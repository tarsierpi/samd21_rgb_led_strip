#![no_main]
#![no_std]

extern crate cortex_m;
extern crate panic_halt;
extern crate xiao_m0 as hal;

use hal::adc::Adc;
use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::entry;
use hal::pac::{CorePeripherals, Peripherals};
//use hal::gpio::{Pa2, Pa4, Pa10, Pa11, Pa8, Pa9, Pb8};
//use hal::gpio::{Input, OpenDrain, Output, PullUp};
use hal::prelude::*;
use hal::pwm::*;
//use hal::pac::gclk::clkctrl::GEN_A::GCLK1;

fn mod1(x : f32) -> f32 {
    return if x < 0.0 {x+1.0} else if x > 1.0 {x-1.0} else {x};
}

fn hue2rgb(p : f32, q: f32, t: f32) -> f32 {
    match mod1(t) {
        t if t < 1.0/6.0 => p + (q - p) * 6.0 * t,
        t if t < 1.0/2.0 => q,
        t if t < 2.0/3.0 => p + (q - p) * ((2.0/3.0) - t) * 6.0,
        _ => p
    }
}

// pass in parameters in range [0.0..1.0)
// https://stackoverflow.com/questions/2353211/hsl-to-rgb-color-conversion
fn hsl2rgb(h : f32, s : f32, lightness : f32) -> (f32, f32, f32) {
    let q: f32 = if lightness < 0.5 {
        lightness * (1.0 + s)
    } else {
        lightness + s - (lightness*s)
    };
    let p = (2.0 * lightness) - q;

    let r = if s == 0.0 {lightness} else {hue2rgb(p, q, h+1.0/3.0)};
    let g = if s == 0.0 {lightness} else {hue2rgb(p, q, h)};
    let b = if s == 0.0 {lightness} else {hue2rgb(p, q, h-1.0/3.0)};

    (r,g,b)
}

fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    return if value >= max {
        max
    } else if value <= min {
        min
    } else {
        value
    }
}

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_external_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut pins = hal::Pins::new(peripherals.PORT);
    let mut delay = Delay::new(core.SYST, &mut clocks);
    
    // The system clock generator at 48Mhz
    let gclk0 = clocks.gclk0();

    // Mapping from Xioa pins to timer configurations.
    //
    // To control the specified pin, configure the pin to "function E" and then control PWM using the 
    // specified timer.
    //
    // From the BSP for Seeeduiono Xiao (should match Xiao datasheet)
    // and SAMD21 datasheet      
    //                               Function "E"         Function "F" 
    // Xiao pin      SAMD pad        Timer Configuration
    // A0/D0/DAC     PA02            No PWM support
    // A1/D1         PA04            TCC0/WO[0]
    // A2/D2         PA10            TCC1/WO[0]
    // A3/D3         PA11            TCC1/WO[1]
    // A4/D4/SDA     PA08            TCC0/WO[0]
    // A5/D5/SCL     PA09            TCC0/WO[1]
    // A6/D6/TX      PB08            TC4/WO[0]
    // A7/D7/RX      PB09            TC4/WO[1]
    // A8/D8/SCK     PA07            TCC1/WO[1]
    // A9/D9/MISO    PA05            TCC0/WO[1]
    // A10/D10/MOSI  PA06            TCC1/WO[0]
    // 'L' LED (0)   PA17            TCC2/WO[1]
    // 'RX' LED (1)  PA18            TC3/WO[0]
    // 'TX' LED (2)  PA19            TC3/WO[1]

    let mut led = pins.led0.into_push_pull_output(&mut pins.port);
    let mut rx_led = pins.led1.into_push_pull_output(&mut pins.port);
    let mut tx_led = pins.led2.into_push_pull_output(&mut pins.port);

    // Initialize all LEDs off
    led.set_high().unwrap();
    tx_led.set_high().unwrap();
    rx_led.set_high().unwrap();

    let _m_btn = pins.a4.into_pull_up_input(&mut pins.port);
    let _power_btn = pins.a5.into_pull_up_input(&mut pins.port);
    //let mut music_note_btn = pins.a6.into_pull_up_input(&mut pins.port);

    //
    // Setup pins controlled by TCC0
    //
    // A1/D1         PA04            TCC0/WO[0]
    // A9/D9/MISO    PA05            TCC0/WO[1]
    // A4/D4/SDA     PA08            TCC0/WO[0]
    // A5/D5/SCL     PA09            TCC0/WO[1]

    // Tell the A1 pin to use the PWM function
    let mut _blue_channel_pin = pins.a1.into_function_e(&mut pins.port);

    // Define some constants to give meaning to TCC0/WO[x]
    let blue_channel = hal::pwm::Channel::_0;
    let _xiao_pin_a4 = hal::pwm::Channel::_0; // mirrors output of previous pin if set to function E
    let _xiao_pin_a5 = hal::pwm::Channel::_1;
    let _xiao_pin_a9 = hal::pwm::Channel::_1; // mirros output of previous pin if set to function E
 
    let tcc0_tcc1_clk = clocks.tcc0_tcc1(&gclk0).unwrap();

    // Set up the PWM parameters
    let mut pwm_tcc0 = Pwm0::new(
        &tcc0_tcc1_clk,
        1.khz(),
        peripherals.TCC0,
        &mut peripherals.PM,
    );

    // And set the duty cycle (initially off)
    let pwm_tcc0_max_duty = pwm_tcc0.get_max_duty();
    pwm_tcc0.set_duty(blue_channel, 0);

    // 
    // Setup pins controlled by TCC1
    //
    // A2/D2         PA10            TCC1/WO[0]
    // A3/D3         PA11            TCC1/WO[1]
    // A8/D8/SCK     PA07            TCC1/WO[1]
    // A10/D10/MOSI  PA06            TCC1/WO[0]
    //

    // Tell the A2 and A3 pins to use the PWM function
    let mut _red_channel_pin = pins.a2.into_function_e(&mut pins.port);
    let mut _green_channel_pin = pins.a3.into_function_e(&mut pins.port);

    // Define some constants to give meaning to TCC0/WO[x]
    let red_channel = hal::pwm::Channel::_0;
    let green_channel = hal::pwm::Channel::_1;
    let _xiao_pin_a8 = hal::pwm::Channel::_1; // Note A3 and A8 would have same output if both configured to PWM.
    let _xiao_pin_a10 = hal::pwm::Channel::_0; // Note A2 and A10 would have same output if both configured to PWM. 

    // match clocks.tcc0_tcc1(&gclk0) {
    //     Some(_) => {tx_led.set_low().unwrap();},
    //     None => {tx_led.set_low().unwrap(); rx_led.set_low().unwrap();},
    // }

    // Set up the PWM parameters
    let mut pwm_tcc1 = Pwm1::new(
        &tcc0_tcc1_clk,
        1.khz(),
        peripherals.TCC1,
        &mut peripherals.PM
    );

    // And set the duty cycle (initially off)
    let pwm_tcc1_max_duty = pwm_tcc1.get_max_duty();
    pwm_tcc1.set_duty(red_channel, 0);
    pwm_tcc1.set_duty(blue_channel, 0);

    // // Setup the TX (blue) LED to be PWM controlled
    // // Xiao pin      SAMD pad        Timer Configuration
    // // 'RX' LED (1)  PA18            TC3/WO[0]
    // // 'TX' LED (2)  PA19            TC3/WO[1]
    // //
    // // Hmmm, te ATSAMD HAL implements the TC PWM using "MPWM" instead of "NPWM",
    // // Upshot of this is we can control WO[1] (the TX LED in this case) but
    // // WO[0] will always have a tiny duty cycle (1 pulse/period).
    // let mut _tx_led_function_e = pins.led2.into_function_e(&mut pins.port);
    // let mut tx_led = Pwm3::new(
    //     &clocks.tcc2_tc3(&gclk0).unwrap(),
    //     1.khz(),
    //     peripherals.TC3,
    //     &mut peripherals.PM,
    // );
    // let tx_rx_led_max_duty = tx_led.get_max_duty();
    // tx_led.set_duty(tx_rx_led_max_duty); // Turn off

    let mut adc = Adc::adc(peripherals.ADC, &mut peripherals.PM, &mut clocks);
    let mut pot_a7 = pins.a7.into_function_b(&mut pins.port);
    let mut pot_a8 = pins.a8.into_function_b(&mut pins.port);

    //
    // Main loop
    //
    loop {

        // 10 bits of resolution whoop whoop
        let adc_divisor = 0x1000 as f32;

        let pot_a7_value_raw : f32 = adc.read(&mut pot_a7).unwrap();
        let pot_a7_value_1: f32 = pot_a7_value_raw / adc_divisor;
        let pot_a7_value: f32 = clamp((pot_a7_value_1 * 1.1) - 0.05, 0.0, 1.0);
        // used for testing, to make sure I am controlling the channel I think I am.
        //pwm_tcc0.set_duty(blue_channel, (pot_a7_value_1 * (pwm_tcc0_max_duty as f32)) as u32);

        let pot_a8_value_raw: f32 = adc.read(&mut pot_a8).unwrap();
        let pot_a8_value_1: f32 = (pot_a8_value_raw as f32) / adc_divisor;
        let pot_a8_value: f32 = clamp((pot_a8_value_1 * 1.1) - 0.05, 0.0, 1.0);
        // used for testing, to make sure I am controlling the channel I think I am.
        //pwm_tcc1.set_duty(red_channel, (pot_a8_value * (pwm_tcc1_max_duty as f32)) as u32);

        let (r, g, b) = hsl2rgb(pot_a8_value, 1.0, pot_a7_value);

        if pot_a8_value <= 0.0 {
            led.set_high().unwrap();
            rx_led.set_low().unwrap();
        } else if pot_a8_value >= 1.0 {
            led.set_low().unwrap();
            rx_led.set_high().unwrap();
        } else {
            led.set_high().unwrap();
            rx_led.set_high().unwrap();
        }

        pwm_tcc1.set_duty(red_channel, (r * (pwm_tcc1_max_duty as f32)) as u32);
        pwm_tcc1.set_duty(green_channel, (g * (pwm_tcc1_max_duty as f32)) as u32);
        pwm_tcc0.set_duty(blue_channel, (b * (pwm_tcc0_max_duty as f32)) as u32);

        delay.delay_ms(50u8);
    }
}
