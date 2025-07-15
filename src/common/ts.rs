use ndarray::Array;
use ndarray::Ix2;

pub trait ToArray{
    fn to_ndarray(&self) -> Array<f32, Ix2>;
}

pub trait Strategy<T>{
    type Output;
    fn on_kline_update(&mut self,input:&T) -> Self::Output;
}