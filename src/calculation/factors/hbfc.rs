use ndarray::{Array, Ix1, Ix2};
use std::error;
pub fn hbfc_one(
    open: &Array<f32, Ix1>,
    close: &Array<f32, Ix1>,
    taker_base: &Array<f32, Ix1>,
) -> Result<Array<f32, Ix1>, Box<dyn error::Error>> {
    let average_price = (open + close) / 2.0;
    Ok(average_price)
}
pub fn ori_hbfc_one(open: &Vec<f32>, close: &Vec<f32>, taker_base: &Vec<f32>) -> Vec<f32> {
    let mut result = Vec::with_capacity(open.len());
    for i in 0..open.len() {
        if close[i] > open[i] {
            let cost: f32 = taker_base[i] * (close[i] - open[i]) / 2.0;
            let price_change: f32 = (close[i] - open[i]) / open[i];
            let factor: f32 = cost / (price_change * 100.0);
            result.push(factor)
        } else {
            result.push(0.0);
        }
    }
    return result;
}
