#![allow(unused_imports)]
use js_sys::Math;
use num_traits::{Float, One, Zero};
use std::ops::{Add, Mul, Sub};

pub trait Lerp<F> {
    /// Interpolate and extrapolate between `self` and `other` using `t` as the parameter.
    ///
    /// At `t == 0.0`, the result is equal to `self`.
    /// At `t == 1.0`, the result is equal to `other`.
    /// At all other points, the result is a mix of `self` and `other`, proportional to `t`.
    ///
    /// `t` is unbounded, so extrapolation and negative interpolation are no problem.
    ///
    /// # Examples
    ///
    /// Basic lerping on floating points:
    ///
    /// ```
    /// use lerp::Lerp;
    ///
    /// let four_32 = 3.0_f32.lerp(5.0, 0.5);
    /// assert_eq!(four_32, 4.0);
    /// let four_64 = 3.0_f64.lerp(5.0, 0.5);
    /// assert_eq!(four_64, 4.0);
    /// ```
    ///
    /// Extrapolation:
    ///
    /// ```
    /// # use lerp::Lerp;
    /// assert_eq!(3.0.lerp(4.0, 2.0), 5.0);
    /// ```
    ///
    /// Negative extrapolation:
    ///
    /// ```
    /// # use lerp::Lerp;
    /// assert_eq!(3.0.lerp(4.0, -1.0), 2.0);
    /// ```
    ///
    /// Reverse interpolation:
    ///
    /// ```
    /// # use lerp::Lerp;
    /// assert_eq!(5.0.lerp(3.0, 0.5), 4.0);
    /// ```
    fn lerp(self, other: Self, t: F) -> Self;

    /// Interpolate between `self` and `other` precisely per the `lerp` function, bounding `t`
    /// in the inclusive range [0..1].
    ///
    /// # Examples
    ///
    /// Bounding on numbers greater than one:
    ///
    /// ```
    /// # use lerp::Lerp;
    /// assert_eq!(3.0.lerp_bounded(4.0, 2.0), 4.0);
    /// ```
    ///
    /// Bounding on numbers less than zero:
    ///
    /// ```
    /// # use lerp::Lerp;
    /// assert_eq!(3.0.lerp_bounded(5.0, -2.0), 3.0);
    /// ```
    fn lerp_bounded(self, other: Self, t: F) -> Self
    where
        Self: Sized,
        F: PartialOrd + Copy + Zero + One,
    {
        let t = match t {
            t if t < F::zero() => F::zero(),
            t if t > F::one() => F::one(),
            t => t,
        };
        self.lerp(other, t)
    }
}

impl<T, F> Lerp<F> for T
where
    T: Copy + Add<Output = T> + Sub<Output = T> + Mul<F, Output = T>,
    F: Float,
{
    fn lerp(self, other: T, t: F) -> T {
        self + ((other - self) * t)
    }
}

pub fn lerp_angle(a: f64, b: f64, t: f64) -> f64 {
    let normal = Vector2 {
        x: a.cos(),
        y: a.sin(),
    };
    let normal2 = Vector2 {
        x: b.cos(),
        y: b.sin(),
    };
    let res = Vector2 {
        x: normal.x.lerp(normal2.x, t),
        y: normal.y.lerp(normal2.y, t),
    };
    res.y.atan2(res.x)
}

/// Holds 2 numbers.
#[derive(Debug, Copy, Clone)]
pub struct Vector2<T: PartialOrd + Copy + Zero + One> {
    pub x: T,
    pub y: T,
}

/// Holds a number and a target number, useful for lerp targets.
#[derive(Debug, Copy, Clone)]
pub struct Scalar<T: Float> {
    pub value: T,
    pub tv: T,
}

impl<T> Scalar<T>
where
    T: Float,
{
    pub fn new(e: T) -> Self {
        Scalar { value: e, tv: e }
    }

    pub fn update(&mut self, e: T) {
        self.value = self.value.lerp(self.tv, e);
    }

    pub fn set_update(&mut self, e: T, t: T) {
        self.tv = e;
        self.update(t);
    }
}

use crate::protocol::Protocol;

pub fn talk<M: Protocol>(ws: &web_sys::WebSocket, data: &M) {
    ws.send_with_u8_array(data.encode().cursor.get_ref().as_slice());
}
