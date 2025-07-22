use ndarray::Array;
use ndarray::Ix2;

pub trait ToArray{
    fn to_ndarray(&self) -> Array<f32, Ix2>;
}

// 重新设计 Strategy trait，使用泛型而不是 trait object
pub trait Strategy<T>: Send + Sync {
    type Output;
    fn on_kline_update(&mut self, input: T) -> Self::Output;
}