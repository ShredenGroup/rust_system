use crate::common::signal::Signal;
pub fn signal_processor(signal: Signal) {
    match signal {
        Signal::Market(market_signal) => {
            println!("Market signal: {:?}", market_signal);
        }
    }
}