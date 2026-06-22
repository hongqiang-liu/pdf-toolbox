#[derive(Debug, Clone, serde::Serialize)]
pub struct ProgressEvent {
    pub task: String,
    pub current: usize,
    pub total: usize,
    pub message: String,
}

impl ProgressEvent {
    pub fn percent(&self) -> u8 {
        if self.total == 0 {
            return 0;
        }
        ((self.current.min(self.total) * 100) / self.total) as u8
    }
}

pub type ProgressSink<'a> = &'a mut dyn FnMut(ProgressEvent);

pub fn emit_progress(
    sink: &mut Option<ProgressSink<'_>>,
    task: &str,
    current: usize,
    total: usize,
    message: impl Into<String>,
) {
    if let Some(callback) = sink.as_deref_mut() {
        callback(ProgressEvent {
            task: task.to_string(),
            current,
            total,
            message: message.into(),
        });
    }
}
