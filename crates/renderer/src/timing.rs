//! Optional native GPU timestamps for capture diagnostics, never a blocking frame loop.
use crate::gpu::Gpu;
use std::{error::Error, sync::mpsc, time::Duration};
const QUERIES: u32 = 8;
pub(crate) struct Timing {
    query: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
}
impl Timing {
    pub fn new(device: &wgpu::Device) -> Self {
        let query = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("capture shadow/world timings"),
            ty: wgpu::QueryType::Timestamp,
            count: QUERIES,
        });
        let buffer = |label, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: QUERIES as u64 * 8,
                usage,
                mapped_at_creation: false,
            })
        };
        Self {
            query,
            resolve: buffer(
                "capture timestamps",
                wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            ),
            readback: buffer(
                "capture timestamp readback",
                wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            ),
        }
    }
    pub fn writes(&self, pass: u32) -> wgpu::RenderPassTimestampWrites<'_> {
        wgpu::RenderPassTimestampWrites {
            query_set: &self.query,
            beginning_of_pass_write_index: Some(pass * 2),
            end_of_pass_write_index: Some(pass * 2 + 1),
        }
    }
    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.resolve_query_set(&self.query, 0..QUERIES, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, QUERIES as u64 * 8);
    }
    pub fn read(&self, gpu: &Gpu) -> Result<[Option<f64>; 4], Box<dyn Error>> {
        let (tx, rx) = mpsc::channel();
        self.readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        gpu.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(15)),
        })?;
        rx.recv()??;
        let bytes = self.readback.slice(..).get_mapped_range();
        let values: &[u64] = bytemuck::cast_slice(&bytes);
        let period = gpu.queue.get_timestamp_period() as f64;
        let result =
            std::array::from_fn(|i| milliseconds(values[i * 2], values[i * 2 + 1], period));
        drop(bytes);
        self.readback.unmap();
        Ok(result)
    }
}

// Some native counters return zero, identical or backwards samples. Never report
// those as a free pass, or manufacture a duration by saturating the subtraction.
fn milliseconds(start: u64, end: u64, period: f64) -> Option<f64> {
    if start == 0 || end <= start || !period.is_finite() || period <= 0. {
        return None;
    }
    Some((end - start) as f64 * period / 1_000_000.)
}
#[cfg(test)]
mod tests {
    use super::milliseconds;
    #[test]
    fn invalid_counters_are_unavailable_not_zero_cost() {
        assert_eq!(milliseconds(100, 2_000_100, 1.), Some(2.));
        for (start, end, period) in [
            (0, 100, 1.),
            (100, 0, 1.),
            (100, 100, 1.),
            (101, 100, 1.),
            (100, 200, 0.),
            (100, 200, f64::NAN),
        ] {
            assert_eq!(milliseconds(start, end, period), None);
        }
    }
}
