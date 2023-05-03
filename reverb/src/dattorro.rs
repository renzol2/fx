use nih_plug::nih_dbg;

use crate::biquad::{BiquadFilter, BiquadFilterType};
use crate::filters::{Allpass, DelayLine};

// Filter constants
const FILTER_Q: f32 = 0.707;
const FILTER_GAIN: f32 = 0.;

// Diffuser constants
const DATTORRO_SAMPLE_RATE: usize = 29761;
const DATTORRO_ALLPASS_1_LENGTH: usize = 142;
const DATTORRO_ALLPASS_2_LENGTH: usize = 107;
const DATTORRO_ALLPASS_3_LENGTH: usize = 379;
const DATTORRO_ALLPASS_4_LENGTH: usize = 277;

// Tank block 1 constants
const DATTORRO_ALLPASS_5_LENGTH: usize = 672;
const DATTORRO_DELAY_1_LENGTH: usize = 4453;
const DATTORRO_ALLPASS_6_LENGTH: usize = 1800;
const DATTORRO_DELAY_2_LENGTH: usize = 3720;

// Tank block 2 constants
const DATTORRO_ALLPASS_5P_LENGTH: usize = 908;
const DATTORRO_DELAY_1P_LENGTH: usize = 4217;
const DATTORRO_ALLPASS_6P_LENGTH: usize = 2656;
const DATTORRO_DELAY_2P_LENGTH: usize = 3163;

struct TankBlock {
    decay_diffuser_1: Allpass,
    delay_1: DelayLine,
    damping_filter: BiquadFilter,
    damping_filter_frequency: f32,
    decay: f32,
    decay_diffuser_2: Allpass,
    delay_2: DelayLine,
}

impl TankBlock {
    pub fn new(
        decay_diffuser_1_length: usize,
        delay_1_length: usize,
        damping_filter_frequency: f32,
        decay: f32,
        decay_diffuser_2_length: usize,
        delay_2_length: usize,
    ) -> TankBlock {
        TankBlock {
            decay_diffuser_1: Allpass::new(decay_diffuser_1_length),
            delay_1: DelayLine::new(delay_1_length),
            damping_filter: BiquadFilter::new(),
            damping_filter_frequency,
            decay,
            decay_diffuser_2: Allpass::new(decay_diffuser_2_length),
            delay_2: DelayLine::new(delay_2_length),
        }
    }

    pub fn initialize(&mut self) {
        self.damping_filter.set_biquad(
            BiquadFilterType::LowPass,
            self.damping_filter_frequency,
            FILTER_Q,
            FILTER_GAIN,
        );
    }

    ///
    /// Returns the sum of tapped outputs and the last delay output.
    ///
    pub fn process(&mut self, input: f32) -> (f32, f32) {
        let decay = self.decay;

        // Begin tank processing
        let decay_diffuser_1_output = self.decay_diffuser_1.tick(input);
        // Get first tap from first diffuser
        let mut output = 0.6 * decay_diffuser_1_output;

        // Delay and damp
        let delay_1_output = self.delay_1.read();
        self.delay_1.write_and_advance(decay_diffuser_1_output);
        // let damping_filter_output = decay * self.damping_filter.process(delay_1_output);

        // Second tap
        output -= 0.6 * delay_1_output; // normally damping out put
        let decay_diffuser_2_output = self.decay_diffuser_2.tick(delay_1_output);

        // Third tap
        output += 0.6 * decay_diffuser_2_output;

        let delay_2_output = self.delay_2.read();
        self.delay_2.write_and_advance(decay_diffuser_2_output);

        (output * decay, delay_2_output)
    }
}

pub struct Dattorro {
    predelay: DelayLine,
    diffuser: [Allpass; 4],
    bandwidth_filter: BiquadFilter,
    tank: (TankBlock, TankBlock),
    tank_signal_1: f32,
    tank_signal_2: f32,
}

///
/// Convert the same length of time (in samples) from one sample rate to another.
///
pub fn sr_convert(old_sr: usize, new_sr: usize, samples: usize) -> usize {
    let seconds = samples as f32 / old_sr as f32;
    (new_sr as f32 * seconds) as usize
}

impl Dattorro {
    pub fn new(
        sample_rate: usize,
        max_predelay_length: usize,
        damping: f32,
        decay: f32,
    ) -> Dattorro {
        let diffuser = [
            Allpass::new(sr_convert(
                DATTORRO_SAMPLE_RATE,
                sample_rate,
                DATTORRO_ALLPASS_1_LENGTH,
            )),
            Allpass::new(sr_convert(
                DATTORRO_SAMPLE_RATE,
                sample_rate,
                DATTORRO_ALLPASS_2_LENGTH,
            )),
            Allpass::new(sr_convert(
                DATTORRO_SAMPLE_RATE,
                sample_rate,
                DATTORRO_ALLPASS_3_LENGTH,
            )),
            Allpass::new(sr_convert(
                DATTORRO_SAMPLE_RATE,
                sample_rate,
                DATTORRO_ALLPASS_4_LENGTH,
            )),
        ];

        let tank_block_1 = TankBlock::new(
            DATTORRO_ALLPASS_5_LENGTH,
            DATTORRO_DELAY_1_LENGTH,
            damping,
            decay,
            DATTORRO_ALLPASS_6_LENGTH,
            DATTORRO_DELAY_2_LENGTH,
        );

        let tank_block_2 = TankBlock::new(
            DATTORRO_ALLPASS_5P_LENGTH,
            DATTORRO_DELAY_1P_LENGTH,
            damping,
            decay,
            DATTORRO_ALLPASS_6P_LENGTH,
            DATTORRO_DELAY_2P_LENGTH,
        );

        Dattorro {
            predelay: DelayLine::new(max_predelay_length),
            diffuser,
            bandwidth_filter: BiquadFilter::new(),
            tank: (tank_block_1, tank_block_2),
            tank_signal_1: 0.0,
            tank_signal_2: 0.0,
        }
    }

    pub fn initialize(&mut self, bandwidth_filter_frequency: f32) {
        self.bandwidth_filter.set_biquad(
            BiquadFilterType::LowPass,
            bandwidth_filter_frequency,
            FILTER_Q,
            FILTER_GAIN,
        );

        self.tank.0.initialize();
        self.tank.1.initialize();
    }

    pub fn process(&mut self, input: (f32, f32), pregain: f32) -> (f32, f32) {
        // Mix to mono, apply pregain and predelay
        let input_mono = ((input.0 + input.1) / 2.0) * pregain;
        let mut processed = self.predelay.read();
        self.predelay.write_and_advance(input_mono);

        // Apply bandwidth, then defuse
        // processed = self.bandwidth_filter.process(processed);
        for allpass in &mut self.diffuser {
            processed = allpass.tick(processed);
        }

        // Pass diffused signal through both tanks
        let (tank_1_output, feedback_signal_1) =
            self.tank.0.process(processed + self.tank_signal_1);
        let (tank_2_output, feedback_signal_2) =
            self.tank.1.process(processed + self.tank_signal_2);

        // Feedback signal through opposite tanks
        let (tank_1_feedback, fedback_signal_1) = self.tank.0.process(feedback_signal_2);
        let (tank_2_feedback, fedback_signal_2) = self.tank.1.process(feedback_signal_1);

        // Sum tapped outputs
        let out_l = 0.6 * tank_1_output - 0.6 * tank_1_feedback;
        let out_r = 0.6 * tank_2_output - 0.6 * tank_2_feedback;

        self.tank_signal_1 = fedback_signal_1;
        self.tank_signal_2 = fedback_signal_2;

        (out_l, out_r)
    }
}

#[cfg(test)]
mod tests {
    use crate::DEFAULT_SAMPLE_RATE;

    use super::*;

    #[test]
    fn sample_rate_conversion_is_correct() {
        let old_sr = DEFAULT_SAMPLE_RATE;
        let new_sr = 44100;
        let samples = DEFAULT_SAMPLE_RATE;
        let result = sr_convert(old_sr, new_sr, samples);
        let expected = new_sr;
        assert_eq!(expected, result);
    }
}
