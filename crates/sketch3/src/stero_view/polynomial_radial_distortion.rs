use glam::*;

pub struct PolynomialRadialDistortion {
    pub coefficients: Vec<f32>,
}

impl PolynomialRadialDistortion {
    // Given a radius (measuring distance from the optical axis of the lens),
    // returns the corresponding distorted radius.
    pub fn distort_radius(&self, r: f32) -> f32 {
        r * self.distortion_factor(r * r)
    }

    // Given a radius (measuring distance from the optical axis of the lens),
    // returns the distortion factor for that radius.
    pub fn distortion_factor(&self, r_squared: f32) -> f32 {
        let mut r_factor = 1.0;
        let mut distortion_factor = 1.0;

        for ki in self.coefficients.iter() {
            r_factor *= r_squared;
            distortion_factor += ki * r_factor;
        }

        return distortion_factor;
    }

    // Given a 2d point p, returns the corresponding distorted point.
    // The units of both the input and output points are tan-angle units,
    // which can be computed as the distance on the screen divided by
    // distance from the virtual eye to the screen. For both the input
    // and output points, the intersection of the optical axis of the lens
    // with the screen defines the origin, the x axis points right, and
    // the y axis points up.
    pub fn distort(&self, p: Vec2) -> Vec2 {
        self.distortion_factor( p.length_squared()) * p
    }

    // Given a 2d point p, returns the point that would need to be passed to
    // Distort to get point p (approximately).
    pub fn distort_inverse(&self, p: Vec2) -> Vec2 {
        let radius = p.length();
        //   if (std::fabs(radius - 0.0f) < std::numeric_limits<float>::epsilon()) {
        //     return std::array<float, 2>();
        //   }

        // Based on the shape of typical distortion curves, |radius| / 2 and
        // |radius| / 3 are good initial guesses for the Secant method that will
        // remain within the intended range of the polynomial.
        let mut r0 = radius / 2.0;
        let mut r1 = radius / 3.0;
        let mut r2;
        let mut dr0 = radius - self.distort_radius(r0);
        let mut dr1;
        while r1 - r0 > 0.0001 {
            dr1 = radius - self.distort_radius(r1);
            r2 = r1 - dr1 * ((r1 - r0) / (dr1 - dr0));
            r0 = r1;
            r1 = r2;
            dr0 = dr1;
        }

        return (r1 / radius) * p;
    }
}
