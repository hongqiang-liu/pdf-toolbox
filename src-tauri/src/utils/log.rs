use crate::core::ProgressEvent;

pub fn cli_progress(event: ProgressEvent) {
    let percent = event.percent();
    eprintln!(
        "[{}] {:>3}% ({}/{}) {}",
        event.task, percent, event.current, event.total, event.message
    );
}
