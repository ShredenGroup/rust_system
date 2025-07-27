use std::collections::HashMap;

pub struct OrderLogicF{
    current_position:HashMap<String,f64>,
    order_history:HashMap<String,Vec<Order>>,
}


