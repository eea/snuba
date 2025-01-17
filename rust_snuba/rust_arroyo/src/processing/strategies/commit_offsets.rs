use crate::processing::strategies::{CommitRequest, MessageRejected, ProcessingStrategy};
use crate::types::{Message, Partition};
use log::info;
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

pub struct CommitOffsets {
    partitions: HashMap<Partition, u64>,
    last_commit_time: SystemTime,
    commit_frequency: Duration,
}
impl <T: Clone>ProcessingStrategy<T> for CommitOffsets {
    fn poll(&mut self) -> Option<CommitRequest> {
        self.commit(false)
    }

    fn submit(&mut self, message: Message<T>) -> Result<(), MessageRejected> {
        for (partition, offset) in message.committable() {
            self.partitions.insert(
                partition,
                offset
            );
        }
        Ok(())
    }

    fn close(&mut self) {}

    fn terminate(&mut self) {}

    fn join(&mut self, _: Option<Duration>) -> Option<CommitRequest> {
        self.commit(true)
    }
}

impl CommitOffsets {
    fn commit(&mut self, force: bool) -> Option<CommitRequest> {
        if SystemTime::now()
            > self
                .last_commit_time
                .checked_add(self.commit_frequency)
                .unwrap()
            || force
        {
            info!("Performing a commit");
            if !self.partitions.is_empty() {
                let ret = Some(CommitRequest {
                    positions: self.partitions.clone(),
                });
                self.partitions.clear();
                self.last_commit_time = SystemTime::now();
                ret
            } else {
                None
            }
        } else {
            None
        }
    }
}

pub fn new(commit_frequency: Duration) -> CommitOffsets {
    CommitOffsets {
        partitions: Default::default(),
        last_commit_time: SystemTime::now(),
        commit_frequency,
    }
}

#[cfg(test)]
mod tests {
    use crate::backends::kafka::types::KafkaPayload;
    use crate::processing::strategies::{commit_offsets, CommitRequest, ProcessingStrategy};
    use crate::types::{Message, Partition, Topic};
    use chrono::DateTime;
    use std::thread::sleep;
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_noop() {
        env_logger::init();
        let partition1 = Partition {
            topic: Topic {
                name: "noop-commit".to_string(),
            },
            index: 0,
        };
        let partition2 = Partition {
            topic: Topic {
                name: "noop-commit".to_string(),
            },
            index: 1,
        };
        let timestamp = DateTime::from(SystemTime::now());
        let m1 = Message {
            partition: partition1.clone(),
            offset: 1000,
            payload: KafkaPayload {
                key: None,
                headers: None,
                payload: None,
            },
            timestamp,
        };
        let m2 = Message {
            partition: partition2.clone(),
            offset: 2000,
            payload: KafkaPayload {
                key: None,
                headers: None,
                payload: None,
            },
            timestamp,
        };

        let mut noop = commit_offsets::new(Duration::from_secs(1));

        let mut commit_req1 = CommitRequest {
            positions: Default::default(),
        };
        commit_req1.positions.insert(
            partition1,
            1001,
        );
        noop.submit(m1).expect("Failed to submit");
        assert_eq!(noop.poll(), None);

        sleep(Duration::from_secs(2));
        assert_eq!(noop.poll(), Some(commit_req1));

        let mut commit_req2 = CommitRequest {
            positions: Default::default(),
        };
        commit_req2.positions.insert(
            partition2,
            2001,
        );
        noop.submit(m2).expect("Failed to submit");
        assert_eq!(noop.poll(), None);
        assert_eq!(noop.join(Some(Duration::from_secs(5))), Some(commit_req2))
    }
}
