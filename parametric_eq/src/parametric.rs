use std::f32::consts::PI;

/// A two pole, two zero filter implementing a parametric EQ.
///
/// Coefficient calculations and input processing code is taken from
/// 'Audio Effects: Theory, Implementation, and Application' by Joshua D. Reiss and
/// Andrew P. McPherson.
pub struct ParametricEqFilter {
    a0: f32,
    a1: f32,
    a2: f32,
    b0: f32,
    b1: f32,
    b2: f32,

    // Filter parameters
    fc: f32,
    q: f32,
    peak_gain: f32,

    // Unit delays
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

// TODO: implement methods for assigning parameters and using the coefficient calculating method
impl ParametricEqFilter {
    pub fn new() -> ParametricEqFilter {
        ParametricEqFilter {
            a0: 0.0,
            a1: 0.0,
            a2: 0.0,
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            fc: 0.5,
            q: 0.707,
            peak_gain: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    pub fn calculate_parametric_coefficients(&mut self) {
        // Get internal values
        let fc = self.fc;
        let q = self.q;
        let gain_factor = self.peak_gain;

        // Calculate intermediate values
        let bandwidth = f32::min(PI * 0.99, fc / q);
        let two_cos_wc = -2.0 * fc.cos();
        let tan_half_bw = (bandwidth / 2.0).tan();
        let g_tan_half_bw = gain_factor * tan_half_bw;
        let sqrt_g = gain_factor.sqrt();

        // Assign coefficients
        self.b0 = sqrt_g + g_tan_half_bw;
        self.b1 = sqrt_g * two_cos_wc;
        self.b2 = sqrt_g - g_tan_half_bw;
        self.a0 = sqrt_g + tan_half_bw;
        self.a1 = sqrt_g * two_cos_wc;
        self.a2 = sqrt_g - tan_half_bw;

        // TODO: normalize coefficients by a0 for time-domain implementation
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let output = self.b0 * input
            + self.b1 * self.x1
            + self.b2 * self.x2
            + self.a1 * self.y1
            + self.a2 * self.y2;
        
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;

        output
    }
}
