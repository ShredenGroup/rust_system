use ndarray::Array;
use ndarray::Ix2;

pub trait ToArray{
    fn to_ndarray(&self) -> Array<f32, Ix2>;
}
