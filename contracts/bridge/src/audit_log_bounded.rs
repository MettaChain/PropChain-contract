pub struct BoundedAuditLog {
    pub max_records: usize,
}

impl Default for BoundedAuditLog {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl BoundedAuditLog {
    pub fn new(max_records: usize) -> Self {
        Self { max_records }
    }

    pub fn push_record<T: Clone>(&self, log: &mut Vec<T>, record: T) {
        if log.len() >= self.max_records {
            log.remove(0);
        }
        log.push(record);
    }
}
