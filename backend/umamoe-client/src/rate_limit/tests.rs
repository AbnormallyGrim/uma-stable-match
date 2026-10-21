use std::sync::{Arc, Mutex as StdMutex};

use super::*;

type Log = Arc<StdMutex<Vec<(&'static str, Duration)>>>;

fn spawn_acquire(
    limiter: &Arc<RateLimiter>,
    priority: Priority,
    label: &'static str,
    start: Instant,
    log: &Log,
) -> tokio::task::JoinHandle<()> {
    let limiter = Arc::clone(limiter);
    let log = Arc::clone(log);
    tokio::spawn(async move {
        limiter.acquire(priority).await;
        log.lock().unwrap().push((label, start.elapsed()));
    })
}

async fn settle() {
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
}

#[tokio::test(start_paused = true)]
async fn first_request_is_immediate() {
    let limiter = RateLimiter::new(Duration::from_millis(100));
    let start = Instant::now();
    limiter.acquire(Priority::Interactive).await;
    assert_eq!(start.elapsed(), Duration::ZERO);
}

#[tokio::test(start_paused = true)]
async fn requests_are_spaced() {
    let limiter = RateLimiter::new(Duration::from_millis(100));
    let start = Instant::now();
    for _ in 0..3 {
        limiter.acquire(Priority::Interactive).await;
    }
    assert_eq!(start.elapsed(), Duration::from_millis(200));
}

#[tokio::test(start_paused = true)]
async fn idle_time_is_not_banked() {
    let limiter = RateLimiter::new(Duration::from_millis(100));
    limiter.acquire(Priority::Interactive).await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let start = Instant::now();
    limiter.acquire(Priority::Interactive).await;
    limiter.acquire(Priority::Interactive).await;
    assert_eq!(start.elapsed(), Duration::from_millis(100));
}

#[tokio::test(start_paused = true)]
async fn zero_interval_never_waits() {
    let limiter = RateLimiter::new(Duration::ZERO);
    let start = Instant::now();
    for _ in 0..5 {
        limiter.acquire(Priority::Interactive).await;
    }
    assert_eq!(start.elapsed(), Duration::ZERO);
}

#[tokio::test(start_paused = true)]
async fn interactive_goes_before_queued_background() {
    let limiter = Arc::new(RateLimiter::new(Duration::from_millis(100)));
    let log = Log::default();
    let start = Instant::now();
    let mut tasks = Vec::new();
    for label in ["bg1", "bg2", "bg3"] {
        tasks.push(spawn_acquire(
            &limiter,
            Priority::Background,
            label,
            start,
            &log,
        ));
        settle().await;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
    tasks.push(spawn_acquire(
        &limiter,
        Priority::Interactive,
        "ui",
        start,
        &log,
    ));
    for task in tasks {
        task.await.unwrap();
    }

    let ms = |n| Duration::from_millis(n);
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            ("bg1", ms(0)),
            ("ui", ms(100)),
            ("bg2", ms(200)),
            ("bg3", ms(300))
        ]
    );
}

#[tokio::test(start_paused = true)]
async fn each_lane_keeps_its_order() {
    let limiter = Arc::new(RateLimiter::new(Duration::from_millis(10)));
    let log = Log::default();
    let start = Instant::now();
    let mut tasks = Vec::new();
    for (priority, label) in [
        (Priority::Background, "bg1"),
        (Priority::Interactive, "ui1"),
        (Priority::Background, "bg2"),
        (Priority::Interactive, "ui2"),
    ] {
        tasks.push(spawn_acquire(&limiter, priority, label, start, &log));
        settle().await;
    }
    for task in tasks {
        task.await.unwrap();
    }

    let order: Vec<_> = log.lock().unwrap().iter().map(|(l, _)| *l).collect();
    assert_eq!(order, vec!["bg1", "ui1", "ui2", "bg2"]);
}

#[tokio::test(start_paused = true)]
async fn lanes_share_the_spacing() {
    let limiter = RateLimiter::new(Duration::from_millis(100));
    let start = Instant::now();
    limiter.acquire(Priority::Background).await;
    limiter.acquire(Priority::Interactive).await;
    limiter.acquire(Priority::Background).await;
    assert_eq!(start.elapsed(), Duration::from_millis(200));
}

#[tokio::test(start_paused = true)]
async fn cancelled_waiters_free_their_place() {
    let limiter = Arc::new(RateLimiter::new(Duration::from_millis(100)));
    let start = Instant::now();
    limiter.acquire(Priority::Interactive).await;

    let waiting = tokio::time::timeout(
        Duration::from_millis(10),
        limiter.acquire(Priority::Interactive),
    )
    .await;
    assert!(waiting.is_err());

    limiter.acquire(Priority::Background).await;
    assert_eq!(start.elapsed(), Duration::from_millis(100));
}

#[tokio::test(start_paused = true)]
async fn concurrent_requests_are_all_spaced() {
    let limiter = Arc::new(RateLimiter::new(Duration::from_millis(100)));
    let log = Log::default();
    let start = Instant::now();
    let tasks: Vec<_> = (0..6)
        .map(|i| {
            let priority = if i % 2 == 0 {
                Priority::Background
            } else {
                Priority::Interactive
            };
            spawn_acquire(&limiter, priority, "req", start, &log)
        })
        .collect();
    for task in tasks {
        task.await.unwrap();
    }

    let mut times: Vec<_> = log.lock().unwrap().iter().map(|(_, t)| *t).collect();
    times.sort();
    let expected: Vec<_> = (0..6).map(|i| Duration::from_millis(100 * i)).collect();
    assert_eq!(times, expected);
}

fn backoff(max_interval_ms: u64, step: f64, every_secs: u64) -> BackoffConfig {
    BackoffConfig {
        max_interval: Duration::from_millis(max_interval_ms),
        recovery_step: step,
        recovery_every: Duration::from_secs(every_secs),
    }
}

fn gap(limiter: &RateLimiter) -> Duration {
    limiter.lock().gap
}

#[tokio::test(start_paused = true)]
async fn retry_after_pauses_every_request() {
    let limiter = RateLimiter::new(Duration::from_millis(100));
    limiter.acquire(Priority::Interactive).await;
    let start = Instant::now();
    limiter.throttled(Some(Duration::from_secs(5)));
    limiter.acquire(Priority::Background).await;
    assert_eq!(start.elapsed(), Duration::from_secs(5));
}

#[tokio::test(start_paused = true)]
async fn each_throttle_halves_the_rate_down_to_the_floor() {
    let limiter = RateLimiter::with_backoff(Duration::from_millis(100), backoff(400, 0.05, 3600));
    limiter.throttled(None);
    assert_eq!(gap(&limiter), Duration::from_millis(200));

    let start = Instant::now();
    limiter.acquire(Priority::Interactive).await;
    limiter.acquire(Priority::Interactive).await;
    assert_eq!(start.elapsed(), Duration::from_millis(200));

    limiter.throttled(None);
    limiter.throttled(None);
    assert_eq!(gap(&limiter), Duration::from_millis(400));
}

#[tokio::test(start_paused = true)]
async fn a_throttle_during_a_pause_does_not_slow_down_again() {
    let limiter = RateLimiter::new(Duration::from_millis(100));
    limiter.throttled(Some(Duration::from_secs(10)));
    limiter.throttled(Some(Duration::from_secs(10)));
    assert_eq!(gap(&limiter), Duration::from_millis(200));

    tokio::time::advance(Duration::from_secs(11)).await;
    limiter.throttled(Some(Duration::from_secs(10)));
    assert_eq!(gap(&limiter), Duration::from_millis(400));
}

#[tokio::test(start_paused = true)]
async fn quiet_periods_recover_the_rate_gradually() {
    let limiter = RateLimiter::with_backoff(Duration::from_millis(100), backoff(30_000, 1.0, 10));
    limiter.throttled(None);
    assert_eq!(gap(&limiter), Duration::from_millis(200));

    tokio::time::advance(Duration::from_secs(9)).await;
    limiter.acquire(Priority::Interactive).await;
    assert_eq!(gap(&limiter), Duration::from_millis(200));

    tokio::time::advance(Duration::from_secs(1)).await;
    limiter.acquire(Priority::Interactive).await;
    let one_step = 1.0 / gap(&limiter).as_secs_f64();
    assert!(
        (one_step - 6.0).abs() < 1e-6,
        "rate after one step: {one_step}"
    );

    tokio::time::advance(Duration::from_secs(60)).await;
    limiter.acquire(Priority::Interactive).await;
    assert_eq!(gap(&limiter), Duration::from_millis(100));
}

#[tokio::test(start_paused = true)]
async fn a_new_throttle_restarts_the_quiet_period() {
    let limiter = RateLimiter::with_backoff(Duration::from_millis(100), backoff(30_000, 1.0, 10));
    limiter.throttled(None);
    tokio::time::advance(Duration::from_secs(8)).await;
    limiter.throttled(None);
    tokio::time::advance(Duration::from_secs(8)).await;
    limiter.acquire(Priority::Interactive).await;
    assert_eq!(gap(&limiter), Duration::from_millis(400));
}

#[tokio::test(start_paused = true)]
async fn zero_interval_only_honours_pauses() {
    let limiter = RateLimiter::new(Duration::ZERO);
    limiter.throttled(Some(Duration::from_secs(1)));
    assert_eq!(gap(&limiter), Duration::ZERO);
    let start = Instant::now();
    limiter.acquire(Priority::Interactive).await;
    limiter.acquire(Priority::Interactive).await;
    assert_eq!(start.elapsed(), Duration::from_secs(1));
}

#[tokio::test(start_paused = true)]
async fn recovery_waits_for_the_pause_to_end() {
    let limiter = RateLimiter::with_backoff(Duration::from_millis(100), backoff(30_000, 1.0, 10));
    limiter.throttled(Some(Duration::from_secs(30)));
    tokio::time::advance(Duration::from_secs(35)).await;
    limiter.acquire(Priority::Interactive).await;
    assert_eq!(gap(&limiter), Duration::from_millis(200));
}
