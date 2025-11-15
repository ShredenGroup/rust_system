use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_system::models::{TradeTick, Exchange, Side, TradingSymbol};
use ta::TradeTickerf64;

fn create_trade_tick() -> TradeTick {
    TradeTick {
        trade_id: 1,
        symbol: TradingSymbol::BTCUSDT,
        price: 50000.0,
        quantity: 0.1,
        side: Side::Buy,
        timestamp: 1000,
        exchange: Exchange::Binance,
    }
}

fn bench_direct_field_access(c: &mut Criterion) {
    let trade = create_trade_tick();
    c.bench_function("direct_field_access", |b| {
        b.iter(|| {
            let qty = black_box(trade.quantity);
            black_box(qty)
        })
    });
}

fn bench_trait_method_access(c: &mut Criterion) {
    let trade = create_trade_tick();
    c.bench_function("trait_method_access", |b| {
        b.iter(|| {
            let qty = black_box(trade.get_trade_quantity());
            black_box(qty)
        })
    });
}

fn bench_direct_field_access_loop(c: &mut Criterion) {
    let trades: Vec<TradeTick> = (0..1000).map(|_| create_trade_tick()).collect();
    c.bench_function("direct_field_access_loop_1000", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for trade in trades.iter() {
                sum += black_box(trade.quantity);
            }
            black_box(sum)
        })
    });
}

fn bench_trait_method_access_loop(c: &mut Criterion) {
    let trades: Vec<TradeTick> = (0..1000).map(|_| create_trade_tick()).collect();
    c.bench_function("trait_method_access_loop_1000", |b| {
        b.iter(|| {
            let mut sum = 0.0;
            for trade in trades.iter() {
                sum += black_box(trade.get_trade_quantity());
            }
            black_box(sum)
        })
    });
}

criterion_group!(
    benches,
    bench_direct_field_access,
    bench_trait_method_access,
    bench_direct_field_access_loop,
    bench_trait_method_access_loop
);
criterion_main!(benches);

