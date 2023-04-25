use std::f32::consts::PI;

pub struct DelayLine {
    circular_buffer: Vec<f32>,
    read_pointer: usize,
    write_pointer: usize,
    delay_time: usize,
    dry_mix: f32,
    wet_mix: f32,
    feedback: f32,
    lfo_phase: f32,
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
            lfo_phase: 0.0,
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
    /// Calculates value at time `t` using cubic interpolation.
    /// https://www.musicdsp.org/en/latest/Other/49-cubic-interpollation.html?highlight=cubic
    ///
    fn get_cubic_interpolated_value_from_buffer(&self, t: f32) -> f32 {
        let buffer = &self.circular_buffer;
        let time = t % buffer.len() as f32;
        let inpos = time.floor() as usize;
        let finpos = time.fract();

        // Get four surrounding samples from buffer
        let xm1 = buffer[if inpos == 0 { buffer.len() } else { inpos } - 1];
        let x0 = buffer[inpos];
        let x1 = buffer[(inpos + 1) % buffer.len()];
        let x2 = buffer[(inpos + 2) % buffer.len()];

        let a = (3. * (x0 - x1) - xm1 + x2) / 2.;
        let b = 2. * x1 + xm1 - (5. * x0 + x2) / 2.;
        let c = (x1 - xm1) / 2.;

        (((a * finpos) + b) * finpos + c) * finpos + x0
    }

    fn get_linear_interpolated_value_from_buffer(&self, t: f32) -> f32 {
        // Use linear interpolation to read a fractional index
        // into the buffer by using the fractional component of
        // the read pointer to adjust weights of adjacent samples
        let time = t % self.circular_buffer.len() as f32;
        let fraction = time - time.floor();
        let previous_sample_index = time.floor() as usize;
        let next_sample_index = (previous_sample_index + 1) % self.circular_buffer.len() as usize;

        fraction * self.circular_buffer[next_sample_index]
            + (1.0 - fraction) * self.circular_buffer[previous_sample_index]
    }

    fn get_interpolated_sample(&self, lfo_width: f32, sample_rate: f32, phase_shift: f32) -> f32 {
        // Recalculate read pointer with respect to write pointer
        let mut lfo_phase = self.lfo_phase + phase_shift;
        if lfo_phase >= 1.0 {
            lfo_phase -= 1.0;
        }
        let phase_component = 2.0 * PI * lfo_phase;
        let current_delay = lfo_width * (0.5 + 0.5 * phase_component.sin());
        let buffer_len = self.circular_buffer.len() as f32;
        let t = self.write_pointer as f32 - (current_delay * sample_rate) as f32 + buffer_len - 3.0;

        self.get_cubic_interpolated_value_from_buffer(t)
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

    pub fn process_with_vibrato(
        &mut self,
        input: f32,
        lfo_frequency: f32,
        vibrato_width: f32,
        sample_rate: f32,
    ) -> f32 {
        let interpolated_sample = self.get_interpolated_sample(vibrato_width, sample_rate, 0.0);

        // Store information in buffer
        self.circular_buffer[self.write_pointer] = input;

        // Increment write pointer at constant rate
        self.write_pointer += 1;

        if self.write_pointer >= self.circular_buffer.len() {
            self.write_pointer = 0;
        }

        // Update LFO phase
        // FIXME: multiplying by 0.5 because the frequency doesn't seem to line up
        self.lfo_phase += 0.5 * lfo_frequency * sample_rate.recip();
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }

        interpolated_sample
    }

    pub fn process_with_flanger(
        &mut self,
        input: f32,
        lfo_frequency: f32,
        vibrato_width: f32,
        sample_rate: f32,
        feedback: f32,
        depth: f32,
    ) -> f32 {
        let interpolated_sample = self.get_interpolated_sample(vibrato_width, sample_rate, 0.0);

        // Store information in buffer
        self.circular_buffer[self.write_pointer] = input + (interpolated_sample * feedback);

        // Increment write pointer at constant rate
        self.write_pointer += 1;

        if self.write_pointer >= self.circular_buffer.len() {
            self.write_pointer = 0;
        }

        // Update LFO phase
        self.lfo_phase += lfo_frequency * sample_rate;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }

        input + depth * interpolated_sample
    }

    pub fn process_with_chorus(
        &mut self,
        input: f32,
        lfo_frequency: f32,
        vibrato_width: f32,
        sample_rate: f32,
        depth: f32,
    ) -> f32 {
        let interpolated_sample = self.get_interpolated_sample(vibrato_width, sample_rate, 0.0);

        // Store information in buffer
        self.circular_buffer[self.write_pointer] = input;

        // Increment write pointer at constant rate
        self.write_pointer += 1;

        if self.write_pointer >= self.circular_buffer.len() {
            self.write_pointer = 0;
        }

        // Update LFO phase
        self.lfo_phase += lfo_frequency * sample_rate;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }

        input + depth * interpolated_sample
    }

    pub fn process_with_stereo_chorus(
        &mut self,
        input: f32,
        lfo_frequency: f32,
        vibrato_width: f32,
        sample_rate: f32,
        depth: f32,
    ) -> (f32, f32) {
        let interpolated_sample_l = self.get_interpolated_sample(vibrato_width, sample_rate, 0.0);
        let interpolated_sample_r = self.get_interpolated_sample(vibrato_width, sample_rate, 0.25);

        // Store information in buffer
        self.circular_buffer[self.write_pointer] = input;

        // Increment write pointer at constant rate
        self.write_pointer += 1;

        if self.write_pointer >= self.circular_buffer.len() {
            self.write_pointer = 0;
        }

        // Update LFO phase
        self.lfo_phase += lfo_frequency * sample_rate;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }

        let output_l = input + depth * interpolated_sample_l;
        let output_r = input + depth * interpolated_sample_r;
        (output_l, output_r)
    }
}
