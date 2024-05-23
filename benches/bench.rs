use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::StatefulWidget;
use ratatui_binary_data_widget::{BinaryDataWidget, BinaryDataWidgetState};

fn renders(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("render");
    group.throughput(Throughput::Elements(1)); // Frames per second

    let buffer_size = Rect::new(0, 0, 100, 100);

    group.bench_function("empty", |bencher| {
        let data = vec![];
        let mut state = BinaryDataWidgetState::default();
        bencher.iter_batched(
            || Buffer::empty(buffer_size),
            |mut buffer| do_render(&mut buffer, &mut state, &data),
            BatchSize::SmallInput,
        );
    });

    for amount in [256, 4096, u16::MAX as usize, 16_000_000] {
        let data = vec![0; amount];
        let mut state = BinaryDataWidgetState::default();
        group.bench_function(format!("{amount}/same"), |bencher| {
            bencher.iter_batched(
                || Buffer::empty(buffer_size),
                |mut buffer| do_render(&mut buffer, &mut state, &data),
                BatchSize::SmallInput,
            );
        });

        let data = (0..=255).cycle().take(amount).collect::<Vec<_>>();
        let mut state = BinaryDataWidgetState::default();
        group.bench_function(format!("{amount}/different"), |bencher| {
            bencher.iter_batched(
                || Buffer::empty(buffer_size),
                |mut buffer| do_render(&mut buffer, &mut state, &data),
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn do_render(buffer: &mut Buffer, state: &mut BinaryDataWidgetState, data: &[u8]) {
    StatefulWidget::render(
        black_box(BinaryDataWidget::new(black_box(data))),
        buffer.area,
        black_box(buffer),
        black_box(state),
    );
}

/// Create flamegraphs with `cargo bench --bench bench -- --profile-time=5`
#[cfg(unix)]
fn profiled() -> Criterion {
    use pprof::criterion::{Output, PProfProfiler};
    Criterion::default().with_profiler(PProfProfiler::new(100, Output::Flamegraph(None)))
}
#[cfg(not(unix))]
fn profiled() -> Criterion {
    Criterion::default()
}

criterion_group! {
    name = benches;
    config = profiled();
    targets = renders
}
criterion_main!(benches);
