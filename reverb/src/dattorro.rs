use crate::biquad::{BiquadFilter, BiquadFilterType, StereoBiquadFilter};
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

    pub fn process(&mut self, input: f32) -> f32 {
        let decay = self.decay;

        // Begin tank processing
        let decay_diffuser_1_output = self.decay_diffuser_1.tick(input);
        // Get first tap from first diffuser
        let mut output = 0.6 * decay_diffuser_1_output;

        // Delay and damp
        let delay_1_output = self.delay_1.read();
        self.delay_1.write_and_advance(decay_diffuser_1_output);
        let damping_filter_output = decay * self.damping_filter.process(delay_1_output);

        // Second tap
        output -= 0.6 * damping_filter_output;
        let decay_diffuser_2_output = self.decay_diffuser_2.tick(damping_filter_output);

        // Third tap
        output += 0.6 * decay_diffuser_2_output;

        let delay_2_output = self.delay_2.read();
        self.delay_2.write_and_advance(delay_2_output);

        output * decay
    }
}

struct Dattorro {
    predelay: DelayLine,
    diffuser: [Allpass; 4],
    bandwidth_filter: StereoBiquadFilter,
    tank: (TankBlock, TankBlock),
}

impl Dattorro {
    ///
    /// Convert the same length of time (in samples) from one sample rate to another.
    ///
    fn sr_convert(old_sr: usize, new_sr: usize, samples: usize) -> usize {
        let seconds = samples as f32 / old_sr as f32;
        (new_sr as f32 * seconds) as usize
    }

    pub fn new(
        sample_rate: usize,
        max_predelay_length: usize,
        damping: f32,
        decay: f32,
    ) -> Dattorro {
        let diffuser = [
            Allpass::new(Dattorro::sr_convert(
                DATTORRO_SAMPLE_RATE,
                sample_rate,
                DATTORRO_ALLPASS_1_LENGTH,
            )),
            Allpass::new(Dattorro::sr_convert(
                DATTORRO_SAMPLE_RATE,
                sample_rate,
                DATTORRO_ALLPASS_2_LENGTH,
            )),
            Allpass::new(Dattorro::sr_convert(
                DATTORRO_SAMPLE_RATE,
                sample_rate,
                DATTORRO_ALLPASS_3_LENGTH,
            )),
            Allpass::new(Dattorro::sr_convert(
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
            bandwidth_filter: StereoBiquadFilter::new(),
            tank: (tank_block_1, tank_block_2),
        }
    }

    pub fn initialize(&mut self, bandwidth_filter_frequency: f32) {
        self.bandwidth_filter.set_biquads(
            BiquadFilterType::LowPass,
            bandwidth_filter_frequency,
            FILTER_Q,
            FILTER_GAIN,
        );

        self.tank.0.initialize();
        self.tank.1.initialize();
    }

    pub fn process(&mut self, input: (f32, f32), pregain: f32) -> (f32, f32) {
        // Mix to mono, apply pregain, and diffuse
        let mut input_mono = (input.0 + input.1) / 2.0;
        input_mono *= pregain;
        let diffused = self
            .diffuser
            .iter_mut()
            .fold(input_mono, |acc, allpass| allpass.tick(acc));

        // Pass diffused signal through both tanks
        let tank_1_output = self.tank.0.process(diffused);
        let tank_2_output = self.tank.1.process(diffused);

        // Feedback signal through opposite tanks
        let tank_1_feedback = self.tank.0.process(tank_2_output);
        let tank_2_feedback = self.tank.1.process(tank_1_output);

        // Sum tapped outputs
        let out_l = tank_1_output + tank_1_feedback;
        let out_r = tank_2_output + tank_2_feedback;

        (out_l, out_r)
    }
}
