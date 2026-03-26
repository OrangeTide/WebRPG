use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::Instant;

pub static SERVER_METRICS: Lazy<ServerMetrics> = Lazy::new(ServerMetrics::new);

/// Lightweight metrics store using atomics and a mutex-protected inner for EWMA/ring buffers.
pub struct ServerMetrics {
    pub start_time: Instant,
    http_request_count: AtomicU64,
    ws_message_count: AtomicU64,
    ws_connection_count: AtomicI64,
    peak_ws_connections_24h: AtomicI64,
    inner: Mutex<MetricsInner>,
}

struct MetricsInner {
    // EWMA values (requests per minute)
    ewma_http_1m: f64,
    ewma_http_5m: f64,
    ewma_http_15m: f64,
    ewma_ws_1m: f64,

    // Previous tick's counter snapshots for computing deltas
    prev_http_count: u64,
    prev_ws_msg_count: u64,

    // Ring buffer for 24h peak tracking (1440 minutes)
    peak_ring: RingBuffer<i64, 1440>,

    // Session cache: round-robin of recent session info
    session_cache: Vec<SessionCacheEntry>,
}

/// A fixed-size circular buffer.
struct RingBuffer<T, const N: usize> {
    buf: [T; N],
    pos: usize,
    filled: bool,
}

impl<T: Default + Copy, const N: usize> RingBuffer<T, N> {
    fn new() -> Self {
        Self {
            buf: [T::default(); N],
            pos: 0,
            filled: false,
        }
    }

    fn push(&mut self, value: T) {
        self.buf[self.pos] = value;
        self.pos = (self.pos + 1) % N;
        if self.pos == 0 {
            self.filled = true;
        }
    }

    fn max(&self) -> T
    where
        T: Ord,
    {
        let end = if self.filled { N } else { self.pos };
        if end == 0 {
            return T::default();
        }
        self.buf[..end].iter().copied().max().unwrap_or_default()
    }
}

/// Ephemeral session info pushed from the server alongside DB writes.
#[derive(Clone, Debug)]
pub struct SessionCacheEntry {
    pub session_id: i32,
    pub name: String,
    pub gm_username: String,
    pub player_count: usize,
    pub created_at: String,
    pub last_active: Instant,
    pub active: bool,
}

/// Snapshot of all metrics for the TUI to render.
#[derive(Clone, Debug)]
pub struct MetricsSnapshot {
    pub uptime_secs: u64,
    pub ws_connections: i64,
    pub peak_connections_24h: i64,
    pub ewma_http_1m: f64,
    pub ewma_http_5m: f64,
    pub ewma_http_15m: f64,
    pub ewma_ws_1m: f64,
    pub recent_sessions: Vec<SessionCacheEntry>,
}

impl ServerMetrics {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            http_request_count: AtomicU64::new(0),
            ws_message_count: AtomicU64::new(0),
            ws_connection_count: AtomicI64::new(0),
            peak_ws_connections_24h: AtomicI64::new(0),
            inner: Mutex::new(MetricsInner {
                ewma_http_1m: 0.0,
                ewma_http_5m: 0.0,
                ewma_http_15m: 0.0,
                ewma_ws_1m: 0.0,
                prev_http_count: 0,
                prev_ws_msg_count: 0,
                peak_ring: RingBuffer::new(),
                session_cache: Vec::new(),
            }),
        }
    }

    /// Increment HTTP request counter (call from middleware).
    pub fn inc_http_requests(&self) {
        self.http_request_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment WebSocket message counter.
    pub fn inc_ws_messages(&self) {
        self.ws_message_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment WebSocket connection count.
    pub fn inc_ws_connections(&self) {
        let new = self.ws_connection_count.fetch_add(1, Ordering::Relaxed) + 1;
        // Update peak if this is a new high
        self.peak_ws_connections_24h
            .fetch_max(new, Ordering::Relaxed);
    }

    /// Decrement WebSocket connection count.
    pub fn dec_ws_connections(&self) {
        self.ws_connection_count.fetch_sub(1, Ordering::Relaxed);
    }

    /// Push or update a session in the cache.
    pub fn push_session(&self, entry: SessionCacheEntry) {
        if let Ok(mut inner) = self.inner.lock() {
            if let Some(existing) = inner
                .session_cache
                .iter_mut()
                .find(|e| e.session_id == entry.session_id)
            {
                existing.name.clone_from(&entry.name);
                existing.gm_username.clone_from(&entry.gm_username);
                existing.player_count = entry.player_count;
                existing.last_active = entry.last_active;
                existing.active = entry.active;
            } else {
                inner.session_cache.push(entry);
                // Keep cache bounded
                if inner.session_cache.len() > 100 {
                    inner.session_cache.remove(0);
                }
            }
        }
    }

    /// Mark a session as inactive in the cache.
    pub fn mark_session_inactive(&self, session_id: i32) {
        if let Ok(mut inner) = self.inner.lock() {
            if let Some(entry) = inner
                .session_cache
                .iter_mut()
                .find(|e| e.session_id == session_id)
            {
                entry.active = false;
                entry.last_active = Instant::now();
            }
        }
    }

    /// Called once per minute to compute EWMA deltas and update 24h peak ring.
    pub fn tick(&self) {
        let http_count = self.http_request_count.load(Ordering::Relaxed);
        let ws_msg_count = self.ws_message_count.load(Ordering::Relaxed);
        let ws_conns = self.ws_connection_count.load(Ordering::Relaxed);

        if let Ok(mut inner) = self.inner.lock() {
            let http_delta = (http_count - inner.prev_http_count) as f64;
            let ws_msg_delta = (ws_msg_count - inner.prev_ws_msg_count) as f64;

            inner.prev_http_count = http_count;
            inner.prev_ws_msg_count = ws_msg_count;

            // EWMA: alpha = 2/(N+1)
            fn update_ewma(current: f64, sample: f64, n: f64) -> f64 {
                let alpha = 2.0 / (n + 1.0);
                alpha * sample + (1.0 - alpha) * current
            }

            inner.ewma_http_1m = update_ewma(inner.ewma_http_1m, http_delta, 1.0);
            inner.ewma_http_5m = update_ewma(inner.ewma_http_5m, http_delta, 5.0);
            inner.ewma_http_15m = update_ewma(inner.ewma_http_15m, http_delta, 15.0);
            inner.ewma_ws_1m = update_ewma(inner.ewma_ws_1m, ws_msg_delta, 1.0);

            // Push current connection count into 24h ring for peak tracking
            inner.peak_ring.push(ws_conns);
        }

        // Update 24h peak from ring buffer
        if let Ok(inner) = self.inner.lock() {
            let ring_max = inner.peak_ring.max();
            self.peak_ws_connections_24h
                .store(ring_max, Ordering::Relaxed);
        }
    }

    /// Return a snapshot of all metrics for rendering.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let uptime_secs = self.start_time.elapsed().as_secs();
        let ws_connections = self.ws_connection_count.load(Ordering::Relaxed);
        let peak_connections_24h = self.peak_ws_connections_24h.load(Ordering::Relaxed);

        let (ewma_http_1m, ewma_http_5m, ewma_http_15m, ewma_ws_1m, recent_sessions) =
            if let Ok(inner) = self.inner.lock() {
                (
                    inner.ewma_http_1m,
                    inner.ewma_http_5m,
                    inner.ewma_http_15m,
                    inner.ewma_ws_1m,
                    inner.session_cache.clone(),
                )
            } else {
                (0.0, 0.0, 0.0, 0.0, Vec::new())
            };

        MetricsSnapshot {
            uptime_secs,
            ws_connections,
            peak_connections_24h,
            ewma_http_1m,
            ewma_http_5m,
            ewma_http_15m,
            ewma_ws_1m,
            recent_sessions,
        }
    }
}
