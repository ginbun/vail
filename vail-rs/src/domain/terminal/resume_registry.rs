use std::collections::HashMap;

/// Reason a new resume session could not be admitted into the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionError {
    /// Global concurrent session cap reached and no detached session is
    /// available to evict (all remaining sessions are actively attached).
    GlobalLimitReached,
    /// Per-user concurrent session cap reached and no detached session of that
    /// user is available to evict.
    PerUserLimitReached,
}

#[derive(Debug)]
struct Entry<T> {
    user_id: i64,
    value: T,
    /// Monotonic sequence captured when the session was last detached.
    /// `None` means the session currently has an attached client connection
    /// and must never be evicted to make room for a new session.
    detached_seq: Option<u64>,
}

/// In-memory registry of keep-alive resume sessions with bounded capacity.
///
/// The registry enforces a global and a per-user concurrency cap. When a new
/// session would exceed a cap, the oldest *detached* session (LRU by detach
/// time) is evicted to make room. Actively attached sessions are never evicted;
/// if no detached session can be freed, admission is rejected so that honest
/// live sessions are preserved and resource usage stays bounded (DoS guard).
#[derive(Debug)]
pub struct SessionRegistry<T> {
    max_global: usize,
    max_per_user: usize,
    seq: u64,
    entries: HashMap<String, Entry<T>>,
}

impl<T: Clone> SessionRegistry<T> {
    pub fn new(max_global: usize, max_per_user: usize) -> Self {
        Self {
            max_global: max_global.max(1),
            max_per_user: max_per_user.max(1),
            seq: 0,
            entries: HashMap::new(),
        }
    }

    /// Update the enforced caps at runtime (config is authoritative). Existing
    /// over-cap entries are not eagerly evicted; the next admission enforces them.
    pub fn set_limits(&mut self, max_global: usize, max_per_user: usize) {
        self.max_global = max_global.max(1);
        self.max_per_user = max_per_user.max(1);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn user_count(&self, user_id: i64) -> usize {
        self.entries
            .values()
            .filter(|entry| entry.user_id == user_id)
            .count()
    }

    pub fn get(&self, id: &str) -> Option<T> {
        self.entries.get(id).map(|entry| entry.value.clone())
    }

    pub fn contains(&self, id: &str) -> bool {
        self.entries.contains_key(id)
    }

    fn next_seq(&mut self) -> u64 {
        self.seq = self.seq.saturating_add(1);
        self.seq
    }

    /// Find the id of the oldest detached entry, optionally restricted to a user.
    fn oldest_detached(&self, user_id: Option<i64>) -> Option<String> {
        self.entries
            .iter()
            .filter(|(_, entry)| user_id.map(|u| entry.user_id == u).unwrap_or(true))
            .filter_map(|(id, entry)| entry.detached_seq.map(|seq| (id.clone(), seq)))
            .min_by_key(|(_, seq)| *seq)
            .map(|(id, _)| id)
    }

    /// Insert a new (attached) session. Evicts the oldest detached sessions as
    /// needed to respect the per-user and global caps. Returns the evicted
    /// values so the caller can tear down their workers / emit audit events.
    pub fn insert(
        &mut self,
        id: String,
        user_id: i64,
        value: T,
    ) -> Result<Vec<T>, AdmissionError> {
        let mut evicted = Vec::new();

        // Respect per-user cap first so a single user cannot crowd out others.
        while self.user_count(user_id) >= self.max_per_user {
            match self.oldest_detached(Some(user_id)) {
                Some(victim) => {
                    if let Some(entry) = self.entries.remove(&victim) {
                        evicted.push(entry.value);
                    }
                }
                None => return Err(AdmissionError::PerUserLimitReached),
            }
        }

        // Then respect the global cap.
        while self.entries.len() >= self.max_global {
            match self.oldest_detached(None) {
                Some(victim) => {
                    if let Some(entry) = self.entries.remove(&victim) {
                        evicted.push(entry.value);
                    }
                }
                None => return Err(AdmissionError::GlobalLimitReached),
            }
        }

        self.entries.insert(
            id,
            Entry {
                user_id,
                value,
                detached_seq: None,
            },
        );
        Ok(evicted)
    }

    /// Mark a session as detached (eligible for LRU eviction).
    pub fn mark_detached(&mut self, id: &str) {
        let seq = self.next_seq();
        if let Some(entry) = self.entries.get_mut(id) {
            entry.detached_seq = Some(seq);
        }
    }

    /// Mark a session as attached (protected from eviction).
    pub fn mark_attached(&mut self, id: &str) {
        if let Some(entry) = self.entries.get_mut(id) {
            entry.detached_seq = None;
        }
    }

    /// Whether the session currently has an attached client connection.
    pub fn is_attached(&self, id: &str) -> bool {
        self.entries
            .get(id)
            .map(|entry| entry.detached_seq.is_none())
            .unwrap_or(false)
    }

    pub fn remove(&mut self, id: &str) -> Option<T> {
        self.entries.remove(id).map(|entry| entry.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_under_limits_does_not_evict() {
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(10, 5);
        assert_eq!(reg.insert("a".into(), 1, 100).unwrap(), Vec::<i32>::new());
        assert_eq!(reg.insert("b".into(), 1, 200).unwrap(), Vec::<i32>::new());
        assert_eq!(reg.len(), 2);
        assert_eq!(reg.user_count(1), 2);
    }

    #[test]
    fn per_user_limit_evicts_oldest_detached() {
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(100, 2);
        reg.insert("a".into(), 1, 1).unwrap();
        reg.insert("b".into(), 1, 2).unwrap();
        // Both attached -> cannot evict -> reject.
        let err = reg.insert("c".into(), 1, 3).unwrap_err();
        assert_eq!(err, AdmissionError::PerUserLimitReached);

        // Detach one; now the new insert evicts the detached one.
        reg.mark_detached("a");
        let evicted = reg.insert("c".into(), 1, 3).unwrap();
        assert_eq!(evicted, vec![1]);
        assert!(!reg.contains("a"));
        assert!(reg.contains("b"));
        assert!(reg.contains("c"));
        assert_eq!(reg.user_count(1), 2);
    }

    #[test]
    fn per_user_limit_evicts_lru_detached_order() {
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(100, 2);
        reg.insert("a".into(), 1, 1).unwrap();
        reg.insert("b".into(), 1, 2).unwrap();
        reg.mark_detached("b"); // detached first (older)
        reg.mark_detached("a"); // detached later (newer)
        // Should evict "b" (oldest detached), not "a".
        let evicted = reg.insert("c".into(), 1, 3).unwrap();
        assert_eq!(evicted, vec![2]);
        assert!(reg.contains("a"));
        assert!(!reg.contains("b"));
    }

    #[test]
    fn per_user_limit_does_not_affect_other_users() {
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(100, 1);
        reg.insert("a".into(), 1, 1).unwrap();
        // user 2 still has room even though user 1 is full.
        assert_eq!(reg.insert("b".into(), 2, 2).unwrap(), Vec::<i32>::new());
        assert_eq!(reg.user_count(1), 1);
        assert_eq!(reg.user_count(2), 1);
    }

    #[test]
    fn global_limit_rejects_when_all_attached() {
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(2, 100);
        reg.insert("a".into(), 1, 1).unwrap();
        reg.insert("b".into(), 2, 2).unwrap();
        let err = reg.insert("c".into(), 3, 3).unwrap_err();
        assert_eq!(err, AdmissionError::GlobalLimitReached);
    }

    #[test]
    fn global_limit_evicts_oldest_detached_across_users() {
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(2, 100);
        reg.insert("a".into(), 1, 1).unwrap();
        reg.insert("b".into(), 2, 2).unwrap();
        reg.mark_detached("b");
        let evicted = reg.insert("c".into(), 3, 3).unwrap();
        assert_eq!(evicted, vec![2]);
        assert!(reg.contains("a"));
        assert!(!reg.contains("b"));
        assert!(reg.contains("c"));
    }

    #[test]
    fn mark_attached_protects_from_eviction() {
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(100, 1);
        reg.insert("a".into(), 1, 1).unwrap();
        reg.mark_detached("a");
        reg.mark_attached("a"); // re-attached, protected again
        let err = reg.insert("b".into(), 1, 2).unwrap_err();
        assert_eq!(err, AdmissionError::PerUserLimitReached);
    }

    #[test]
    fn set_limits_applies_to_next_admission() {
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(100, 100);
        reg.insert("a".into(), 1, 1).unwrap();
        reg.set_limits(100, 1);
        let err = reg.insert("b".into(), 1, 2).unwrap_err();
        assert_eq!(err, AdmissionError::PerUserLimitReached);
    }

    #[test]
    fn remove_frees_capacity() {
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(100, 1);
        reg.insert("a".into(), 1, 1).unwrap();
        assert_eq!(reg.remove("a"), Some(1));
        assert!(reg.is_empty());
        assert_eq!(reg.insert("b".into(), 1, 2).unwrap(), Vec::<i32>::new());
    }

    #[test]
    fn eviction_targets_detached_even_if_newer_than_attached() {
        // Attached sessions must be protected regardless of age: the only
        // detached entry is evicted even though it is the most recently active.
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(3, 100);
        reg.insert("a".into(), 1, 1).unwrap(); // oldest, stays attached
        reg.insert("b".into(), 2, 2).unwrap(); // stays attached
        reg.insert("c".into(), 3, 3).unwrap(); // newest, will be detached
        reg.mark_detached("c");
        let evicted = reg.insert("d".into(), 4, 4).unwrap();
        assert_eq!(evicted, vec![3]);
        assert!(reg.contains("a"));
        assert!(reg.contains("b"));
        assert!(!reg.contains("c"));
        assert!(reg.contains("d"));
        assert_eq!(reg.len(), 3);
    }

    #[test]
    fn attach_detach_cycle_keeps_counts_consistent() {
        // Counts are derived from the entry map, so attach/detach/remove cycles
        // can never produce stale, negative, or leaked counters.
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(100, 100);
        reg.insert("a".into(), 7, 1).unwrap();
        reg.insert("b".into(), 7, 2).unwrap();
        assert_eq!(reg.user_count(7), 2);
        assert_eq!(reg.len(), 2);

        reg.mark_detached("a");
        reg.mark_attached("a");
        reg.mark_detached("b");
        // State flips never change the count.
        assert_eq!(reg.user_count(7), 2);
        assert_eq!(reg.len(), 2);

        assert_eq!(reg.remove("b"), Some(2));
        assert_eq!(reg.user_count(7), 1);
        assert_eq!(reg.len(), 1);
        // Removing a non-existent id is a no-op and does not underflow.
        assert_eq!(reg.remove("missing"), None);
        assert_eq!(reg.user_count(7), 1);
    }

    #[test]
    fn evicts_multiple_detached_to_satisfy_per_user_cap() {
        // When the cap is lowered well below the current count, a single
        // admission may need to evict more than one detached session.
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(100, 100);
        reg.insert("a".into(), 1, 1).unwrap();
        reg.insert("b".into(), 1, 2).unwrap();
        reg.insert("c".into(), 1, 3).unwrap();
        reg.mark_detached("a");
        reg.mark_detached("b");
        reg.set_limits(100, 2);
        // Inserting a 4th while cap is 2 must free two detached entries.
        let mut evicted = reg.insert("d".into(), 1, 4).unwrap();
        evicted.sort();
        assert_eq!(evicted, vec![1, 2]);
        assert!(reg.contains("c"));
        assert!(reg.contains("d"));
        assert_eq!(reg.user_count(1), 2);
    }

    #[test]
    fn reserved_session_survives_global_capacity_pressure() {
        // D1: a detached session that is then re-marked attached (reserved during
        // a resume handshake) must NOT be evicted by a concurrent new admission,
        // even under global capacity pressure. With no other evictable detached
        // entry, the new admission is rejected instead.
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(1, 100);
        reg.insert("a".into(), 1, 1).unwrap();
        reg.mark_detached("a");
        // Resume reserves the session inside the critical section.
        reg.mark_attached("a");
        let err = reg.insert("b".into(), 2, 2).unwrap_err();
        assert_eq!(err, AdmissionError::GlobalLimitReached);
        assert!(reg.contains("a"));
        assert!(reg.is_attached("a"));
    }

    #[test]
    fn is_attached_acts_as_reservation_flag() {
        // D2: once a resume reserves the session (mark_attached), a second
        // concurrent resume observes is_attached == true and must back off.
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(10, 10);
        reg.insert("a".into(), 1, 1).unwrap();
        reg.mark_detached("a");
        // First resume: observes not attached, reserves it.
        assert!(!reg.is_attached("a"));
        reg.mark_attached("a");
        // Second resume: observes the reservation and rejects.
        assert!(reg.is_attached("a"));
    }

    #[test]
    fn is_attached_reflects_state() {
        let mut reg: SessionRegistry<i32> = SessionRegistry::new(10, 10);
        reg.insert("a".into(), 1, 1).unwrap();
        assert!(reg.is_attached("a"));
        reg.mark_detached("a");
        assert!(!reg.is_attached("a"));
        assert!(!reg.is_attached("missing"));
    }
}
