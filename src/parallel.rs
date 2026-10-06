//! A dependency-free work queue for running tasks on several threads.

use std::collections::VecDeque;
use std::sync::Mutex;

/// Runs `task` for every item, on a pool of threads, and returns the results in
/// the original order. Each task receives its item's index so it can report
/// progress; failures are values, not panics.
pub fn map<T, R, F>(items: Vec<T>, task: F) -> Vec<R>
where
    T: Send,
    R: Send,
    F: Fn(usize, T) -> R + Sync,
{
    let total = items.len();
    if total == 0 {
        return Vec::new();
    }

    let queue = Mutex::new(items.into_iter().enumerate().collect::<VecDeque<_>>());
    let collected = Mutex::new(Vec::with_capacity(total));
    let workers = worker_count(total);

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let Some((index, item)) = queue.lock().expect("queue poisoned").pop_front()
                    else {
                        break;
                    };
                    let result = task(index, item);
                    collected
                        .lock()
                        .expect("results poisoned")
                        .push((index, result));
                }
            });
        }
    });

    let mut results = collected.into_inner().expect("results poisoned");
    results.sort_by_key(|(index, _)| *index);
    results.into_iter().map(|(_, result)| result).collect()
}

/// Number of threads to use: enough to keep the network busy, but never more
/// than there are items.
fn worker_count(total: usize) -> usize {
    let cores = std::thread::available_parallelism().map_or(4, |n| n.get());
    (cores * 2).clamp(1, 8).min(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_order_and_visits_every_item() {
        let results = map(vec![1, 2, 3, 4, 5], |index, item| index * 100 + item);
        assert_eq!(results, vec![1, 102, 203, 304, 405]);
    }

    #[test]
    fn handles_an_empty_input() {
        let results: Vec<usize> = map(Vec::<usize>::new(), |_, item| item);
        assert!(results.is_empty());
    }

    #[test]
    fn never_exceeds_the_item_count() {
        assert_eq!(worker_count(1), 1);
        assert!(worker_count(1000) <= 8);
    }
}
