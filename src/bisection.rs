/// Bisection method for finding roots of a function.
pub struct Bisection {
    pub lower: f64,
    pub upper: f64,
    pub epsilon: f64,
    pub maxIterations: f64,
}

impl Bisection {
    /// Creates a new bisection object.
    pub fn new(
        lower: f64,
        upper: f64,
        epsilon: f64,
        maxIterations: f64,
    ) -> Self {
        Self {
            upper,
            lower,
            epsilon,
            maxIterations,
        }
    }

    /// Finds the root of the function `fx` between `lower` and `upper` with a maximum error of `epsilon`.
    pub fn bisection<F>(&self, fx: F) -> f64 where F: Fn(f64) -> f64 {
        let mut root = 0.0;
        let mut distance = self.upper - self.lower;
        let mut iterations = 0.0;
        let mut upper_temp = self.upper;
        let mut lower_temp = self.lower;

        while distance > self.epsilon && iterations < self.maxIterations {
            root = (lower_temp + upper_temp) / 2.0;
            let output = fx(root);

            if output * fx(lower_temp) <= 0.0 {
                upper_temp = root;
            } else {
                lower_temp = root;
            }

            distance = upper_temp - lower_temp;
            iterations += 1.0;
        }

        root
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn find_root() {
        // basic polynomial function
        let fx = |x: f64| x.powi(3) - x.powi(2) + 2.0;
        let bisection = super::Bisection::new(-200.0, 300.0, 0.0001, 1000.0);
        let root = bisection.bisection(fx);
        assert!((root - -1.0).abs() < 0.0001); // about 1, but floating point error!
    }
}
