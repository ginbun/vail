use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    GapExceeded {
        requested_offset: u64,
        earliest_offset: u64,
    },
    InvalidOffset {
        requested_offset: u64,
        current_offset: u64,
    },
}

#[derive(Debug, Clone)]
struct ReplayChunk {
    start_offset: u64,
    end_offset: u64,
    payload: String,
}

#[derive(Debug, Clone)]
pub struct ReplayRingBuffer {
    max_bytes: usize,
    total_bytes: usize,
    next_offset: u64,
    chunks: VecDeque<ReplayChunk>,
}

impl ReplayRingBuffer {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes: max_bytes.max(1),
            total_bytes: 0,
            next_offset: 0,
            chunks: VecDeque::new(),
        }
    }

    pub fn next_offset(&self) -> u64 {
        self.next_offset
    }

    pub fn earliest_offset(&self) -> u64 {
        self.chunks
            .front()
            .map(|chunk| chunk.start_offset)
            .unwrap_or(self.next_offset)
    }

    pub fn append(&mut self, payload: &str) -> u64 {
        let bytes = payload.as_bytes().len();
        let start_offset = self.next_offset;
        self.next_offset = self.next_offset.saturating_add(bytes as u64);

        if bytes == 0 {
            return start_offset;
        }

        self.total_bytes = self.total_bytes.saturating_add(bytes);
        self.chunks.push_back(ReplayChunk {
            start_offset,
            end_offset: self.next_offset,
            payload: payload.to_string(),
        });

        while self.total_bytes > self.max_bytes {
            if let Some(chunk) = self.chunks.pop_front() {
                self.total_bytes = self.total_bytes.saturating_sub(chunk.payload.as_bytes().len());
            } else {
                break;
            }
        }

        start_offset
    }

    pub fn replay_from(&self, offset: u64) -> Result<Vec<String>, ReplayError> {
        if offset > self.next_offset {
            return Err(ReplayError::InvalidOffset {
                requested_offset: offset,
                current_offset: self.next_offset,
            });
        }

        let earliest = self.earliest_offset();
        if offset < earliest {
            return Err(ReplayError::GapExceeded {
                requested_offset: offset,
                earliest_offset: earliest,
            });
        }

        let mut out = Vec::new();
        for chunk in self.chunks.iter().filter(|chunk| chunk.end_offset > offset) {
            let skip = if offset > chunk.start_offset {
                (offset - chunk.start_offset) as usize
            } else {
                0
            };
            if skip >= chunk.payload.len() {
                continue;
            }
            match chunk.payload.get(skip..) {
                Some(slice) => out.push(slice.to_string()),
                None => {
                    return Err(ReplayError::InvalidOffset {
                        requested_offset: offset,
                        current_offset: self.next_offset,
                    });
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay_ring_buffer_tracks_offsets() {
        let mut buf = ReplayRingBuffer::new(1024);
        assert_eq!(buf.append("abc"), 0);
        assert_eq!(buf.append("de"), 3);
        assert_eq!(buf.next_offset(), 5);
        assert_eq!(buf.earliest_offset(), 0);
        assert_eq!(
            buf.replay_from(3).expect("replay"),
            vec!["de".to_string()]
        );
    }

    #[test]
    fn replay_ring_buffer_returns_gap_error_when_offset_evicted() {
        let mut buf = ReplayRingBuffer::new(6);
        buf.append("abcd");
        buf.append("efgh");
        let err = buf.replay_from(0).expect_err("gap");
        assert_eq!(
            err,
            ReplayError::GapExceeded {
                requested_offset: 0,
                earliest_offset: 4
            }
        );
    }

    #[test]
    fn replay_ring_buffer_rejects_future_offset() {
        let mut buf = ReplayRingBuffer::new(1024);
        buf.append("abc");
        let err = buf.replay_from(10).expect_err("invalid offset");
        assert_eq!(
            err,
            ReplayError::InvalidOffset {
                requested_offset: 10,
                current_offset: 3
            }
        );
    }

    #[test]
    fn replay_ring_buffer_slices_within_chunk() {
        let mut buf = ReplayRingBuffer::new(1024);
        buf.append("abcde");
        assert_eq!(
            buf.replay_from(3).expect("replay"),
            vec!["de".to_string()]
        );
    }

    #[test]
    fn replay_ring_buffer_does_not_duplicate_across_chunks() {
        let mut buf = ReplayRingBuffer::new(1024);
        buf.append("abc");
        buf.append("def");
        assert_eq!(
            buf.replay_from(2).expect("replay"),
            vec!["c".to_string(), "def".to_string()]
        );
    }

    #[test]
    fn replay_ring_buffer_enforces_max_bytes() {
        let mut buf = ReplayRingBuffer::new(8);
        buf.append("12345678");
        buf.append("AB");
        assert_eq!(buf.earliest_offset(), 8);
        assert_eq!(buf.next_offset(), 10);
        let total: usize = buf
            .replay_from(8)
            .expect("replay")
            .iter()
            .map(|s| s.len())
            .sum();
        assert!(total <= 8);
    }

    #[test]
    fn replay_ring_buffer_empty_offset_returns_empty() {
        let mut buf = ReplayRingBuffer::new(1024);
        buf.append("abc");
        assert!(buf.replay_from(3).expect("replay").is_empty());
    }
}
