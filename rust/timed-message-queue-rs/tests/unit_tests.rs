use std::time::{Duration, Instant};
use timed_message_queue_rs::port::TimedMessageQueue;

#[test]
fn evicts_oldest_when_full_and_expires_stale() {
    let mut q = TimedMessageQueue::new(2, 100);
    let t0 = Instant::now();
    q.add_message_at("first", t0);
    q.add_message_at("second", t0 + Duration::from_millis(50));
    q.add_message_at("third", t0 + Duration::from_millis(80));

    let items: Vec<&str> = q
        .active_messages_at(t0 + Duration::from_millis(90))
        .collect();
    assert_eq!(items, ["second", "third"]);

    let items_later: Vec<&str> = q
        .active_messages_at(t0 + Duration::from_millis(160))
        .collect();
    assert_eq!(items_later, ["third"]);

    let items_end: Vec<&str> = q
        .active_messages_at(t0 + Duration::from_millis(200))
        .collect();
    assert!(items_end.is_empty());
}
