use super::*;

#[tokio::test(start_paused = true)]
async fn first_request_is_immediate() {
    let limiter = RateLimiter::new(Duration::from_millis(100));
    let start = Instant::now();
    limiter.acquire().await;
    assert_eq!(start.elapsed(), Duration::ZERO);
}

#[tokio::test(start_paused = true)]
async fn requests_are_spaced() {
    let limiter = RateLimiter::new(Duration::from_millis(100));
    let start = Instant::now();
    for _ in 0..3 {
        limiter.acquire().await;
    }
    assert_eq!(start.elapsed(), Duration::from_millis(200));
}

#[tokio::test(start_paused = true)]
async fn idle_time_is_not_banked() {
    let limiter = RateLimiter::new(Duration::from_millis(100));
    limiter.acquire().await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let start = Instant::now();
    limiter.acquire().await;
    limiter.acquire().await;
    assert_eq!(start.elapsed(), Duration::from_millis(100));
}

#[tokio::test(start_paused = true)]
async fn zero_interval_never_waits() {
    let limiter = RateLimiter::new(Duration::ZERO);
    let start = Instant::now();
    for _ in 0..5 {
        limiter.acquire().await;
    }
    assert_eq!(start.elapsed(), Duration::ZERO);
}
