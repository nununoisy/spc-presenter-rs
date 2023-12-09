use std::convert::TryInto;

pub trait SimdSum<T = Self>: Sized {
    fn simd_sum<const LANES: usize>(values: &[Self]) -> Self;
}

macro_rules! simd_sum_impl {
    ($t: ty, $zero: expr) => {
        impl SimdSum for $t {
            fn simd_sum<const LANES: usize>(values: &[Self]) -> Self {
                let chunks = values.chunks_exact(LANES);
                let remainder = chunks.remainder();

                let sum = chunks.fold([$zero; LANES], |mut acc, chunk| {
                    let chunk: [Self; LANES] = chunk.try_into().unwrap();
                    for i in 0..LANES {
                        acc[i] += chunk[i];
                    }
                    acc
                });

                let remainder: Self = remainder.iter().copied().sum();

                let mut reduced = $zero;
                for i in 0..LANES {
                    reduced += sum[i];
                }
                reduced + remainder
            }
        }
    }
}

simd_sum_impl!(f64, 0.0_f64);
simd_sum_impl!(f32, 0.0_f32);

pub trait SimdSumAdapter<T: SimdSum> {
    fn simd_sum<const LANES: usize>(&self) -> T;
}

impl<T: SimdSum, C: ?Sized + AsRef<[T]>> SimdSumAdapter<T> for C {
    fn simd_sum<const LANES: usize>(&self) -> T {
        T::simd_sum::<LANES>(self.as_ref())
    }
}
