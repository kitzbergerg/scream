#![no_main]
#![no_std]

use cortex_m_rt::entry;
use lsm303agr::{AccelOutputDataRate, Lsm303agr, Measurement};
use microbit::{
    hal::{
        clocks::Clocks,
        gpio,
        prelude::OutputPin,
        pwm,
        time::Hertz,
    },
    Board,
};
use microbit::hal::Twim;
use microbit::pac::twim0::frequency::FREQUENCY_A;
use panic_rtt_target as _;
use rtt_target::{rprintln, rtt_init_print};

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = Board::take().unwrap();

    // NB: The LF CLK pin is used by the speaker
    let _clocks = Clocks::new(board.CLOCK)
        .enable_ext_hfosc()
        .set_lfclk_src_synth()
        .start_lfclk();

    let mut speaker_pin = board.speaker_pin.into_push_pull_output(gpio::Level::High);
    let _ = speaker_pin.set_low();

    // Use the PWM peripheral to generate a waveform for the speaker
    let speaker = pwm::Pwm::new(board.PWM0);
    speaker
        // output the waveform on the speaker pin
        .set_output_pin(pwm::Channel::C0, speaker_pin.degrade())
        // Use prescale by 16 to achive darker sounds
        .set_prescaler(pwm::Prescaler::Div16)
        // Initial frequency
        .set_period(Hertz(500u32))
        // Configure for up and down counter mode
        .set_counter_mode(pwm::CounterMode::UpAndDown)
        // Set maximum duty cycle
        .set_max_duty(32767)
        // enable PWM
        .enable();

    speaker
        .set_seq_refresh(pwm::Seq::Seq0, 0)
        .set_seq_end_delay(pwm::Seq::Seq0, 0)
        .set_period(Hertz(500u32));

    let i2c = { Twim::new(board.TWIM0, board.i2c_internal.into(), FREQUENCY_A::K100) };
    let mut sensor = Lsm303agr::new_with_i2c(i2c);
    sensor.init().unwrap();
    sensor.set_accel_odr(AccelOutputDataRate::Hz50).unwrap();

    let mut data = AccelerationData::default();
    loop {
        let measurement = sensor.accel_data().unwrap();
        data.add_measurement(measurement);

        if data.is_falling() {
            let max_duty = speaker.max_duty();
            speaker.set_duty_on_common(max_duty / 2);
        } else {
            speaker.set_duty_on_common(0);
        }
    }
}


const DATA_LENGTH: usize = 32;

struct AccelerationData {
    x: [i32; DATA_LENGTH],
    y: [i32; DATA_LENGTH],
    z: [i32; DATA_LENGTH],
    i: usize,
}

impl Default for AccelerationData {
    fn default() -> Self {
        AccelerationData {
            x: [0; DATA_LENGTH],
            y: [0; DATA_LENGTH],
            z: [0; DATA_LENGTH],
            i: 0,
        }
    }
}

impl AccelerationData {
    fn add_measurement(&mut self, m: Measurement) {
        self.x[self.i] = m.x;
        self.y[self.i] = m.y;
        self.z[self.i] = m.z;
        self.i = (self.i + 1) % DATA_LENGTH;
    }

    fn sum(&self) -> (i32, i32, i32) {
        let x = self.x.iter().sum();
        let y = self.y.iter().sum();
        let z = self.z.iter().sum();
        (x, y, z)
    }

    fn is_falling(&self) -> bool {
        let (x, y, z) = self.sum();
        x.abs() < 200 * DATA_LENGTH as i32 &&
            y.abs() < 200 * DATA_LENGTH as i32 &&
            z.abs() < 200 * DATA_LENGTH as i32
    }
}