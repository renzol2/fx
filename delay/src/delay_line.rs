pub struct DelayLine {
    circular_buffer: Vec<f32>,
    read_pointer: usize,
    write_pointer: usize,
    delay_time: usize,
    dry_mix: f32,
    wet_mix: f32,
    feedback: f32,
}

impl DelayLine {
    pub fn new(buffer_length: usize) -> DelayLine {
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

    pub fn process(&mut self, input: f32) -> f32 {
        let buffer_length = self.circular_buffer.len();
        // let output = self.dry_mix * input + self.wet_mix * self.circular_buffer[self.read_pointer];

        // Write input signal and feedback signal into buffer
        // self.circular_buffer[wp] =
        //     input + (self.circular_buffer[self.read_pointer] * self.feedback);

        // FIXME: doesn't prevent zipper noise
        // Use linear interpolation to calculate output value
        let rp = (self.write_pointer as f32 - self.delay_time as f32 + buffer_length as f32 - 3.0)
            % buffer_length as f32;
        let fraction = rp.fract();
        let prev_sample = rp.floor() as usize;
        let next_sample = (prev_sample + 1) % buffer_length;
        let interpolated_sample = fraction * self.circular_buffer[next_sample]
            + (1.0 - fraction) * self.circular_buffer[prev_sample];
        let output = self.dry_mix * input + self.wet_mix * interpolated_sample;

        // Write input signal and feedback signal into buffer
        self.circular_buffer[self.write_pointer] =
            input + (self.circular_buffer[rp as usize] * self.feedback);

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
