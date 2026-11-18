#![forbid(unsafe_code)]

use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub const DEFAULT_MAX_MESSAGES: usize = 13_371_337;
pub const DEFAULT_ERROR_MESSAGE_MS: u64 = 10_000;

#[derive(Debug, Clone)]
pub struct TimedMessage {
    pub message: String,
    release_time: Instant,
}

#[derive(Debug, Clone)]
pub struct TimedMessageQueue {
    messages: VecDeque<TimedMessage>,
    max_messages: usize,
    duration: Duration,
}

impl Default for TimedMessageQueue {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_MESSAGES, DEFAULT_ERROR_MESSAGE_MS)
    }
}

impl TimedMessageQueue {
    pub fn new(max_messages: usize, duration_ms: u64) -> Self {
        Self {
            messages: VecDeque::new(),
            max_messages,
            duration: Duration::from_millis(duration_ms),
        }
    }

    pub fn add_message(&mut self, message: impl Into<String>) {
        self.add_message_at(message, Instant::now());
    }

    pub fn add_message_at(&mut self, message: impl Into<String>, now: Instant) {
        if self.max_messages == 0 {
            return;
        }
        if self.messages.len() == self.max_messages {
            self.messages.pop_front();
        }
        self.messages.push_back(TimedMessage {
            message: message.into(),
            release_time: now + self.duration,
        });
    }

    pub fn active_messages(&mut self) -> impl Iterator<Item = &str> {
        self.active_messages_at(Instant::now())
    }

    pub fn active_messages_at(&mut self, now: Instant) -> impl Iterator<Item = &str> {
        while let Some(front) = self.messages.front() {
            if front.release_time < now {
                self.messages.pop_front();
            } else {
                break;
            }
        }
        self.messages.iter().map(|m| m.message.as_str())
    }
}
