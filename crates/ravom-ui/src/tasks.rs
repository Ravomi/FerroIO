//! Background job runner: one long-running job (build/flash) at a time, results
//! streamed back to the UI thread over a channel.

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;

pub enum TaskEvent {
    Log(String),
    Finished,
}

pub struct TaskRunner {
    tx: Sender<TaskEvent>,
    rx: Receiver<TaskEvent>,
    running: Option<String>,
}

impl Default for TaskRunner {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            running: None,
        }
    }
}

impl TaskRunner {
    /// Label of the job currently running, if any.
    pub fn running(&self) -> Option<&str> {
        self.running.as_deref()
    }

    /// Start `job` unless another job is running; returns whether it started.
    /// `job` receives a logger that streams lines to the console while it runs.
    pub fn spawn<F>(&mut self, label: &str, job: F) -> bool
    where
        F: FnOnce(&mut dyn FnMut(&str)) -> Result<String, String> + Send + 'static,
    {
        if self.running.is_some() {
            return false;
        }
        self.running = Some(label.to_string());
        let tx = self.tx.clone();
        let label = label.to_string();
        thread::spawn(move || {
            let log_tx = tx.clone();
            let mut log = move |line: &str| {
                let _ = log_tx.send(TaskEvent::Log(line.to_string()));
            };
            let msg = match job(&mut log) {
                Ok(msg) => format!("{label} OK: {msg}"),
                Err(err) => format!("{label} error: {err}"),
            };
            let _ = tx.send(TaskEvent::Log(msg));
            let _ = tx.send(TaskEvent::Finished);
        });
        true
    }

    /// Drain pending events, clearing the running flag when a job finishes.
    pub fn poll(&mut self) -> Vec<TaskEvent> {
        let events: Vec<TaskEvent> = self.rx.try_iter().collect();
        if events.iter().any(|e| matches!(e, TaskEvent::Finished)) {
            self.running = None;
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn runs_one_job_at_a_time_and_reports_completion() {
        let mut runner = TaskRunner::default();
        let (gate_tx, gate_rx) = mpsc::channel::<()>();
        assert!(runner.spawn("Build", move |log| {
            let _ = gate_rx.recv();
            log("compiling");
            Ok("done".to_string())
        }));
        assert_eq!(runner.running(), Some("Build"));
        assert!(
            !runner.spawn("Flash", |_| Ok(String::new())),
            "second job must be rejected"
        );

        gate_tx.send(()).expect("release job");
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut logs = Vec::new();
        while runner.running().is_some() && Instant::now() < deadline {
            for event in runner.poll() {
                if let TaskEvent::Log(l) = event {
                    logs.push(l);
                }
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(runner.running(), None);
        assert_eq!(logs, ["compiling", "Build OK: done"]);
    }
}
