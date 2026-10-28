//! # Async Channel
//!
//! In this exercise, you will use `tokio::sync::mpsc` async channels to implement producer-consumer pattern.
//!
//! ## Concepts
//! - `tokio::sync::mpsc::channel` creates bounded async channels
//! - Async `send` and `recv`
//! - Channel closing mechanism (receiver returns None after all senders are dropped)

use tokio::sync::mpsc;

/// Async producer-consumer:
/// - Create a producer task that sends each element from items sequentially
/// - Create a consumer task that receives all elements and collects them into Vec for return
///
/// Hint: Set channel capacity to items.len().max(1)
pub async fn producer_consumer(items: Vec<String>) -> Vec<String> {
    // TODO: Create channel with mpsc::channel
    // TODO: Spawn producer task: iterate through items, send each one
    // TODO: Spawn consumer task: loop recv until channel closes, collect results
    // TODO: Wait for consumer to complete and return results

    let capacity = items.len().max(1);
    let (tx, mut rx) = mpsc::channel(capacity);

    let producer = tokio::spawn(async move {
        for item in items {
            tx.send(item).await.unwrap();
        }
    });

    let consumer = tokio::spawn(async move {
        let mut result = Vec::new();

        // 没错，使用 while let 是对的
        while let Some(item) = rx.recv().await {
            result.push(item);
        }

        result
    });

    let result = consumer.await.unwrap();
    let _ = producer.await;

    result
}

/// Fan‑in pattern: multiple producers, one consumer.
/// Create `n_producers` producers, each sending `"producer {id}: message"`.
/// Consumer collects all messages, sorts them, and returns.
pub async fn fan_in(n_producers: usize) -> Vec<String> {
    // TODO: Create mpsc channel
    // TODO: Spawn n_producers producer tasks
    //       Each sends format!("producer {id}: message")
    // TODO: Drop the original sender (important! otherwise channel won't close)
    // TODO: Consumer loops receiving, collects and sorts

    let (tx, mut rx) = mpsc::channel(n_producers.max(1));

    for id in 0..n_producers {
        let tx = tx.clone();

        tokio::spawn(async move {
            let message = format!("producer {id}: message");
            let _ = tx.send(message).await; // 发送后的忽略返回值，在这种小 demo 当中确实没有意义
        });
    }


    //  drop 掉原来的 tx 否则管道一直不结束
    drop(tx);

    let mut result = Vec::with_capacity(n_producers);

    while let Some(message) = rx.recv().await {
        result.push(message);
    }

    result.sort();
    result


}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_producer_consumer() {
        let items = vec!["hello".into(), "async".into(), "world".into()];
        let result = producer_consumer(items.clone()).await;
        assert_eq!(result, items);
    }

    #[tokio::test]
    async fn test_producer_consumer_empty() {
        let result = producer_consumer(vec![]).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_fan_in() {
        let result = fan_in(3).await;
        assert_eq!(
            result,
            vec![
                "producer 0: message",
                "producer 1: message",
                "producer 2: message",
            ]
        );
    }

    #[tokio::test]
    async fn test_fan_in_single() {
        let result = fan_in(1).await;
        assert_eq!(result, vec!["producer 0: message"]);
    }
}
