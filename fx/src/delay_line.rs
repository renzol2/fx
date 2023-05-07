use std::f32::consts::PI;

///
/// Performs cubic interpolation given four adjacent samples
/// https://www.musicdsp.org/en/latest/Other/49-cubic-interpollation.html?highlight=cubic
///
/// # Arguments
/// * `fpos` - fractional component of position
/// * `xm1` - value corresponding to `x[n-1]`
/// * `x0` - value corresponding to `x[n]`
/// * `x1` - value corresponding to `x[n+1]`
/// * `x2` - value corresponding to `x[n+2]`
///
fn get_cubic_interpolated_value(fpos: f32, xm1: f32, x0: f32, x1: f32, x2: f32) -> f32 {
    let a = (3. * (x0 - x1) - xm1 + x2) / 2.;
    let b = 2. * x1 + xm1 - (5. * x0 + x2) / 2.;
    let c = (x1 - xm1) / 2.;

    (((a * fpos) + b) * fpos + c) * fpos + x0
}

pub struct DelayLine {
    circular_buffer: Vec<f32>,
    read_pointer: usize,
    write_pointer: usize,
    delay_time: usize,
    dry_mix: f32,
    wet_mix: f32,
    feedback: f32,
    sample_rate: usize,
}

impl DelayLine {
    pub fn new(buffer_length: usize, sample_rate: usize) -> DelayLine {
        let mut circular_buffer = Vec::with_capacity(buffer_length);
        circular_buffer.resize(buffer_length, 0.0);
        DelayLine {
            circular_buffer,
            read_pointer: 0,
            write_pointer: 0,
            dry_mix: 0.0,
            wet_mix: 1.0,
            feedback: 0.5,
            delay_time: 0,
            sample_rate,
        }
    }

    ///
    /// Changes the read pointer position based on a given delay time.
    ///
    /// # Arguments
    /// * `delay_time` - The desired delay time, in milliseconds
    /// * `sample_rate` - The sample rate of the system
    ///
    pub fn set_delay_time(&mut self, delay_time: f32, sample_rate: f32) {
        let wp = self.write_pointer as f32;
        let buffer_length = self.circular_buffer.len();
        let delay_in_samples = (delay_time / 1000.0) * sample_rate;
        self.delay_time = delay_in_samples as usize;
        self.read_pointer = (wp - delay_in_samples + buffer_length as f32) as usize % buffer_length;
    }

    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback;
    }

    pub fn set_dry_wet(&mut self, dry_mix: f32, wet_mix: f32) {
        self.dry_mix = dry_mix;
        self.wet_mix = wet_mix;
    }

    ///
    /// Resize and clear the circular buffer.
    ///
    /// # Arguments
    /// - `new_size`: the new size of the circular buffer, in samples
    ///
    pub fn resize_buffer(&mut self, new_size: usize) {
        self.circular_buffer.resize(new_size, 0.0);
    }

    ///
    /// Resize and clear the circular buffer.
    ///
    /// # Arguments
    /// - `new_size`: the new size of the circular buffer, in samples
    /// - `sample_rate`: the new sample rate
    ///
    pub fn resize_buffer_with_sample_rate(&mut self, new_size: usize, sample_rate: usize) {
        self.sample_rate = sample_rate;
        self.circular_buffer.resize(new_size, 0.0);
    }

    ///
    /// Get fractional read time into buffer
    ///
    fn get_read_time(&self, lfo_phase: f32, lfo_width: f32) -> f32 {
        let phase_component = 2.0 * PI * lfo_phase;
        let current_delay = lfo_width * (0.5 + 0.5 * phase_component.sin());
        let buffer_len = self.circular_buffer.len() as f32;

        self.write_pointer as f32 - (current_delay * self.sample_rate as f32) as f32 + buffer_len
            - 3.0
    }

    ///
    /// Calculates value at time `t` using cubic interpolation.
    ///
    fn get_cubic_interpolated_value_from_buffer(&self, t: f32, buffer: &Vec<f32>) -> f32 {
        let time = t % buffer.len() as f32;
        let inpos = time.floor() as usize;
        let finpos = time.fract();

        // Get four surrounding samples from buffer
        let xm1 = buffer[if inpos == 0 { buffer.len() } else { inpos } - 1];
        let x0 = buffer[inpos];
        let x1 = buffer[(inpos + 1) % buffer.len()];
        let x2 = buffer[(inpos + 2) % buffer.len()];

        get_cubic_interpolated_value(finpos, xm1, x0, x1, x2)
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let buffer_length = self.circular_buffer.len();
        let t = (self.write_pointer as f32 - self.delay_time as f32 + buffer_length as f32 - 3.0)
            % buffer_length as f32;
        let interpolated_sample =
            self.get_cubic_interpolated_value_from_buffer(t, &self.circular_buffer);
        let output = self.dry_mix * input + self.wet_mix * interpolated_sample;

        // Write input signal and feedback signal into buffer
        self.circular_buffer[self.write_pointer] =
            input + (self.circular_buffer[t as usize] * self.feedback);

        self.read_pointer += 1;
        self.write_pointer += 1;

        if self.read_pointer >= self.circular_buffer.len() {
            self.read_pointer = 0;
        }
        if self.write_pointer >= self.circular_buffer.len() {
            self.write_pointer = 0;
        }

        output
    }
}

