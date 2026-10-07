use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DeletionFailure {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct DeletionResult {
    pub deleted: Vec<String>,
    pub failures: Vec<DeletionFailure>,
}

impl DeletionResult {
    pub fn failed(&mut self, path: impl Into<String>, reason: impl Into<String>) {
        self.failures.push(DeletionFailure {
            path: path.into(),
            reason: reason.into(),
        });
    }
}
