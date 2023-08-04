/// Bisection method for finding roots of a function.
/// lower - lower bound of the search space
/// upper - upper bound of the search space
/// epsilon - maximum error between root and discovered value
/// max_iter - maximum number of iterations to perform
pub struct Bisection {
    pub lower: f64,
    pub upper: f64,
    pub epsilon: f64,
    pub max_iter: f64,
}

/// Bisection, or binary search, is a method for finding roots of a function.
/// It works by taking a lower and upper bound, and then finding the midpoint between them.
/// If the midpoint is the root, then we are done. Otherwise, we check if the midpoint is
/// greater than or less than the root. If it is greater than the root, then we set the
/// midpoint as the new upper bound. If it is less than the root, then we set the midpoint
/// as the new lower bound. We then repeat this process until we find the root, or until
/// we reach the maximum number of iterations.
#[allow(unused)]
impl Bisection {
    /// Creates a new bisection object.
    pub fn new(lower: f64, upper: f64, epsilon: f64, max_iter: f64) -> Self {
        Self {
            upper,
            lower,
            epsilon,
            max_iter,
        }
    }

    /// Finds the root of the function `fx` between `lower` and `upper` with a maximum error of `epsilon`.
    /// fx - function to find the root of.
    pub fn bisection<F>(&self, fx: F) -> f64
    where
        F: Fn(f64) -> f64,
    {
        let mut root = 0.0;
        let mut distance = self.upper - self.lower;
        let mut iterations = 0.0;
        let mut upper_temp = self.upper;
        let mut lower_temp = self.lower;

        while distance > self.epsilon && iterations < self.max_iter {
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

        println!(
            "found root at distance {} less than epsilon {} in {} iterations",
            distance, self.epsilon, iterations
        );
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
