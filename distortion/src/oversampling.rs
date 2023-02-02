/// A second-order allpass filter.
/// 
/// Adapted for non-SIMD from Fredemus in va-filter, which is licensed under GPL 3.0:
/// https://github.com/Fredemus/va-filter
pub struct AllpassFilter {
    pub a: f32,

    pub x0: f32,
    pub x1: f32,
    pub x2: f32,

    pub y0: f32,
    pub y1: f32,
    pub y2: f32,
}

impl Default for AllpassFilter {
    fn default() -> Self {
        Self {
            a: 0.0,
            x0: 0.0,
            x1: 0.0,
            x2: 0.0,
            y0: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }
}

impl AllpassFilter {
    fn new(coefficient: f32) -> AllpassFilter {
        AllpassFilter {
            a: coefficient,
            x0: 0.0,
            x1: 0.0,
            x2: 0.0,
            y0: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn process(&mut self, input_sample: f32) -> f32 {
        // Shuffle inputs
        self.x2 = self.x1; 
        self.x1 = self.x0;
        self.x0 = input_sample;

        // Shuffle outputs
        self.y2 = self.y1;
        self.y1 = self.y0;

        let output = self.x2 + ((input_sample - self.y2) * self.a);
        self.y0 = output;

        output
    }
}
 