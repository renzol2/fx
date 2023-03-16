use nih_plug::prelude::*;
use std::f32::consts::PI;

#[derive(Enum, Debug, PartialEq, Eq)]
pub enum BiquadFilterType {
    LowPass,
    HighPass,
    BandPass,
    Notch,
    ParametricEQ,
    LowShelf,
    HighShelf,
}

// Biquad filter code from: https://www.earlevel.com/main/2012/11/26/biquad-c-source-code/
pub struct BiquadFilter {
    // Filter type & coefficients
    filter_type: BiquadFilterType,
    a0: f32,
    a1: f32,
    a2: f32,
    b1: f32,
    b2: f32,

    // Filter parameters
    fc: f32,
    q: f32,
    peak_gain: f32,

    // Unit delays
    z1: f32,
    z2: f32,
}

impl BiquadFilter {
    pub fn new() -> BiquadFilter {
        let mut bqf = BiquadFilter {
            filter_type: BiquadFilterType::LowPass,
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            b1: 0.0,
            b2: 0.0,
            fc: 0.5,
            q: 0.707,
            peak_gain: 0.0,
            z1: 0.0,
            z2: 0.0,
        };
        bqf.set_filter_type(BiquadFilterType::LowPass);
        bqf
    }

    pub fn set_filter_type(&mut self, filter_type: BiquadFilterType) {
        self.filter_type = filter_type;
        self.calculate_biquad_coefficients();
    }

    pub fn set_q(&mut self, q: f32) {
        self.q = q;
        self.calculate_biquad_coefficients();
    }

    pub fn set_fc(&mut self, fc: f32) {
        self.fc = fc;
        self.calculate_biquad_coefficients();
    }

    pub fn set_peak_gain(&mut self, peak_gain: f32) {
        self.peak_gain = peak_gain;
        self.calculate_biquad_coefficients();
    }

    pub fn set_biquad(&mut self, filter_type: BiquadFilterType, fc: f32, q: f32, peak_gain: f32) {
        self.filter_type = filter_type;
        self.q = q;
        self.fc = fc;
        self.set_peak_gain(peak_gain)
    }

    pub fn calculate_biquad_coefficients(&mut self) {
        let v = 10.0_f32.powf(self.peak_gain.abs() / 20.0);
        let k = (PI * self.fc).tan();

        // FIXME: cut for parametric, low shelf, and high self is not cutting at all
        match self.filter_type {
            BiquadFilterType::LowPass => {
                let norm = (1.0 + k / self.q + k * k).recip();
                self.a0 = k * k * norm;
                self.a1 = 2.0 * self.a0;
                self.a2 = self.a0;
                self.b1 = 2.0 * (k * k - 1.0) * norm;
                self.b2 = (1.0 - k / self.q + k * k) * norm;
            }
            BiquadFilterType::HighPass => {
                let norm = (1.0 + k / self.q + k * k).recip();
                self.a0 = 1.0 * norm;
                self.a1 = -2.0 * self.a0;
                self.a2 = self.a0;
                self.b1 = 2.0 * (k * k - 1.0) * norm;
                self.b2 = (1.0 - k / self.q + k * k) * norm;
            }
            BiquadFilterType::BandPass => {
                let norm = (1.0 + k / self.q + k * k).recip();
                self.a0 = k / self.q * norm;
                self.a1 = 0.0;
                self.a2 = -self.a0;
                self.b1 = 2.0 * (k * k - 1.0) * norm;
                self.b2 = (1.0 - k / self.q + k * k) * norm;
            }
            BiquadFilterType::Notch => {
                let norm = (1.0 + k / self.q + k * k).recip();
                self.a0 = (1.0 + k * k) * norm;
                self.a1 = 2.0 * (k * k - 1.0) * norm;
                self.a2 = self.a0;
                self.b1 = self.a1;
                self.b2 = (1.0 - k / self.q + k * k) * norm;
            }
            BiquadFilterType::ParametricEQ => {
                if self.peak_gain >= 0.0 {
                    // boost
                    let norm = (1.0 + self.q.recip() * k + k * k).recip();
                    self.a0 = (1.0 + v / self.q * k + k * k) * norm;
                    self.a1 = 2.0 * (k * k - 1.0) * norm;
                    self.a2 = (1.0 - v / self.q * k + k * k) * norm;
                    self.b1 = self.a1;
                    self.b2 = (1.0 - self.q.recip() * k + k * k) * norm;
                } else {
                    // cut
                    let norm = (1.0 + self.q.recip() * k + k * k).recip();
                    self.a0 = (1.0 + self.q.recip() * k + k * k) * norm;
                    self.a1 = 2.0 * (k * k - 1.0) * norm;
                    self.a2 = (1.0 - self.q.recip() * k + k * k) * norm;
                    self.b1 = self.a1;
                    self.b2 = (1.0 - v / self.q * k + k * k) * norm;
                }
            }
            BiquadFilterType::LowShelf => {
                if self.peak_gain >= 0.0 {
                    // boost
                    let norm = (1.0 + 2.0_f32.sqrt() * k + k * k).recip();
                    self.a0 = (1.0 + (2.0 * v).sqrt() * k + v * k * k) * norm;
                    self.a1 = 2.0 * (v * k * k - 1.0) * norm;
                    self.a2 = (1.0 - (2.0 * v).sqrt() * k + v * k * k) * norm;
                    self.b1 = 2.0 * (k * k - 1.0) * norm;
                    self.b2 = (1.0 - 2.0_f32.sqrt() * k + k * k) * norm;
                } else {
                    // cut
                    let norm = (1.0 + (2.0 * v).sqrt() * k + v * k * k).recip();
                    self.a0 = (1.0 + 2.0_f32.sqrt() * k + k * k) * norm;
                    self.a1 = 2.0 * (k * k - 1.0) * norm;
                    self.a2 = (1.0 - 2.0_f32.sqrt() * k + k * k) * norm;
                    self.b1 = 2.0 * (v * k * k - 1.0) * norm;
                    self.b2 = (1.0 - (2.0 * v).sqrt() * k + v * k * k) * norm;
                }
            }
            BiquadFilterType::HighShelf => {
                if self.peak_gain >= 0.0 {
                    // boost
                    let norm = (1.0 + 2.0_f32.sqrt() * k + k * k).recip();
                    self.a0 = (v + (2.0 * v).sqrt() * k + k * k) * norm;
                    self.a1 = 2.0 * (k * k - v) * norm;
                    self.a2 = (v - (2.0 * v).sqrt() * k + k * k) * norm;
                    self.b1 = 2.0 * (k * k - 1.0) * norm;
                    self.b2 = (1.0 - 2.0_f32.sqrt() * k + k * k) * norm;
                } else {
                    // cut
                    let norm = (v + (2.0 * v).sqrt() * k + k * k).recip();
                    self.a0 = (1.0 + 2.0_f32.sqrt() * k + k * k) * norm;
                    self.a1 = 2.0 * (k * k - 1.0) * norm;
                    self.a2 = (1.0 - 2.0_f32.sqrt() * k + k * k) * norm;
                    self.b1 = 2.0 * (k * k - v) * norm;
                    self.b2 = (v - (2.0 * v).sqrt() * k + k * k) * norm;
                }
            }
        }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let output = input * self.a0 + self.z1;
        self.z1 = input * self.a1 + self.z2 - self.b1 * output;
        self.z2 = input * self.a2 - self.b2 * output;
        output
    }
}
