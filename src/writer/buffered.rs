use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, PoisonError};

use tracing_subscriber::fmt::MakeWriter;

#[derive(Debug)]
struct State {
    events: VecDeque<Vec<u8>>,
    front_offset: usize,
    queued_bytes: usize,
    dropped: u64,
}

#[derive(Clone, Debug)]
pub struct BufferedWriter {
    state: Arc<Mutex<State>>,
    capacity: usize,
}

#[derive(Debug)]
pub struct BufferedWriteGuard {
    state: Arc<Mutex<State>>,
    buffer: Vec<u8>,
    capacity: usize,
    overflow: bool,
}

impl BufferedWriter {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                events: VecDeque::new(),
                front_offset: 0,
                queued_bytes: 0,
                dropped: 0,
            })),
            capacity,
        }
    }

    #[must_use]
    pub fn drain(&self, max_bytes: usize) -> Vec<u8> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let mut output = Vec::with_capacity(max_bytes.min(state.queued_bytes));

        while output.len() < max_bytes {
            let Some(event) = state.events.front() else {
                break;
            };
            let event_len = event.len();
            let offset = state.front_offset;
            let drained = (max_bytes - output.len()).min(event_len - offset);
            let Some(bytes) = event.get(offset..offset + drained) else {
                break;
            };
            output.extend_from_slice(bytes);
            state.front_offset += drained;
            state.queued_bytes -= drained;
            if state.front_offset == event_len {
                state.events.pop_front();
                state.front_offset = 0;
            }
        }

        drop(state);
        output
    }

    #[must_use]
    pub fn dropped(&self) -> u64 {
        self.state
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .dropped
    }
}

impl<'a> MakeWriter<'a> for BufferedWriter {
    type Writer = BufferedWriteGuard;

    fn make_writer(&'a self) -> Self::Writer {
        BufferedWriteGuard {
            state: Arc::clone(&self.state),
            buffer: Vec::new(),
            capacity: self.capacity,
            overflow: false,
        }
    }
}

impl Write for BufferedWriteGuard {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !self.overflow {
            if bytes.len() > self.capacity.saturating_sub(self.buffer.len()) {
                self.overflow = true;
                self.buffer.clear();
            } else {
                self.buffer.extend_from_slice(bytes);
            }
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for BufferedWriteGuard {
    fn drop(&mut self) {
        if self.buffer.is_empty() && !self.overflow {
            return;
        }

        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if self.overflow || self.buffer.len() > self.capacity.saturating_sub(state.queued_bytes) {
            state.dropped = state.dropped.saturating_add(1);
        } else {
            state.queued_bytes += self.buffer.len();
            state.events.push_back(std::mem::take(&mut self.buffer));
        }
    }
}
