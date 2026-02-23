use crusty_core::PluginRegistry;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<PluginRegistry>,
    pub plugins_base: PathBuf,
    pub jobs: Arc<JobState>,
}

#[derive(Clone)]
pub struct JobState {
    inner: Arc<RwLock<JobStateInner>>,
}

impl Default for JobState {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(JobStateInner::default())),
        }
    }
}

#[derive(Default)]
struct JobStateInner {
    status: HashMap<String, String>,
    output: HashMap<String, Vec<u8>>,
    error: HashMap<String, String>,
}

impl JobState {
    pub fn set_status(&self, job_id: &str, status: JobStatus) {
        let s = match status {
            JobStatus::Pending => "pending",
            JobStatus::Running => "running",
            JobStatus::Completed => "completed",
            JobStatus::Failed => "failed",
        };
        self.inner.write().unwrap().status.insert(job_id.to_string(), s.to_string());
    }

    pub fn set_completed(&self, job_id: &str, output: Vec<u8>) {
        let mut g = self.inner.write().unwrap();
        g.status.insert(job_id.to_string(), "completed".to_string());
        g.output.insert(job_id.to_string(), output);
    }

    pub fn set_failed(&self, job_id: &str, err: String) {
        let mut g = self.inner.write().unwrap();
        g.status.insert(job_id.to_string(), "failed".to_string());
        g.error.insert(job_id.to_string(), err);
    }

    pub fn get_status(&self, job_id: &str) -> Option<String> {
        self.inner.read().unwrap().status.get(job_id).cloned()
    }

    pub fn get_output(&self, job_id: &str) -> Option<Vec<u8>> {
        self.inner.read().unwrap().output.get(job_id).cloned()
    }
}

#[allow(dead_code)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
}
