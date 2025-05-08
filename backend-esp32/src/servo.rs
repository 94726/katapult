use anyhow::Result;
use esp_idf_hal::ledc::LedcDriver;

pub struct Servo<'d> {
    pwm: LedcDriver<'d>,
    min_us: u32,
    max_us: u32,
    current_angle: i32,
}

impl<'d> Servo<'d> {
    /// Create a new servo on the given pin with 50Hz PWM
    pub fn new(pwm: LedcDriver<'d>, min_us: u32, max_us: u32) -> Result<Self> {
        let mut servo = Self {
            pwm,
            min_us,
            max_us,
            current_angle: 0,
        };
        servo.pwm.enable()?;
        servo.set_angle(0)?;
        Ok(servo)
    }

    /// Convenience method for standard servos (500–2000 µs range)
    pub fn standard(pwm: LedcDriver<'d>) -> Result<Self> {
        Self::new(pwm, 500, 2000)
    }

    /// Set servo angle in degrees (0–180 or -90 to 90, depending on mapping)
    pub fn set_angle(&mut self, angle: i32) -> Result<()> {
        self.current_angle = angle.clamp(-90, 90);
        let duty_us = self.map_angle_to_us(self.current_angle);
        self.pwm.set_duty(duty_us)?;
        println!("duty: {}, current_angle: {}", duty_us, self.current_angle);
        Ok(())
    }
    pub fn get_angle(&self) -> i32 {
        self.current_angle
    }

    fn map_angle_to_us(&self, angle: i32) -> u32 {
        let min_angle = -90;
        let max_angle = 90;

        ((angle - min_angle) as u32) * (self.max_us - self.min_us)
            / ((max_angle - min_angle) as u32)
            + self.min_us
    }
}
