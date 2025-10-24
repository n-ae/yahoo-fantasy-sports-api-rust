//! Yahoo Fantasy Sports SDK - Rust Implementation  
//! Core API client with authentication, rate limiting, and caching

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use serde::{Deserialize, Serialize};

/// Main SDK client
#[derive(Debug)]
pub struct YahooFantasyClient {
    consumer_key: String,
    consumer_secret: String,
    access_token: Option<String>,
    access_token_secret: Option<String>,
    base_url: String,
    rate_limiter: Arc<RateLimiter>,
    cache: Arc<Cache>,
}

impl YahooFantasyClient {
    /// Create a new Yahoo Fantasy client
    pub fn new(consumer_key: String, consumer_secret: String) -> Self {
        Self {
            consumer_key,
            consumer_secret,
            access_token: None,
            access_token_secret: None,
            base_url: "https://fantasysports.yahooapis.com/fantasy/v2".to_string(),
            rate_limiter: Arc::new(RateLimiter::new()),
            cache: Arc::new(Cache::new()),
        }
    }

    /// Set OAuth access tokens
    pub fn set_tokens(&mut self, access_token: String, access_token_secret: String) {
        self.access_token = Some(access_token);
        self.access_token_secret = Some(access_token_secret);
    }

    /// Check if client is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.access_token.is_some() && self.access_token_secret.is_some()
    }

    /// Get all available games (mock implementation for testing)
    pub fn get_games(&self) -> Result<Vec<Game>, Box<dyn std::error::Error + Send + Sync>> {
        self.rate_limiter.wait_for_request();
        
        let games = vec![
            Game {
                game_key: "nfl.2024".to_string(),
                name: "NFL Football".to_string(),
                code: "nfl".to_string(),
                season: 2024,
            },
            Game {
                game_key: "nba.2024".to_string(),
                name: "NBA Basketball".to_string(),
                code: "nba".to_string(),
                season: 2024,
            },
            Game {
                game_key: "mlb.2024".to_string(),
                name: "MLB Baseball".to_string(),
                code: "mlb".to_string(),
                season: 2024,
            },
        ];

        self.rate_limiter.record_request();
        Ok(games)
    }

    /// Get user's leagues for a specific game (mock implementation)
    pub fn get_leagues(&self, game_key: &str) -> Result<Vec<League>, Box<dyn std::error::Error + Send + Sync>> {
        self.rate_limiter.wait_for_request();
        
        let _ = game_key;
        let leagues = vec![
            League {
                league_key: "423.l.12345".to_string(),
                name: "My Test League".to_string(),
                num_teams: 12,
                current_week: 15,
            }
        ];

        self.rate_limiter.record_request();
        Ok(leagues)
    }
}

/// Game data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub game_key: String,
    pub name: String,
    pub code: String,
    pub season: i32,
}

/// League data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct League {
    pub league_key: String,
    pub name: String,
    pub num_teams: i32,
    pub current_week: i32,
}

/// Rate limiter using token bucket algorithm
#[derive(Debug)]
pub struct RateLimiter {
    state: Mutex<RateLimiterState>,
}

#[derive(Debug)]
struct RateLimiterState {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
    requests_count: u64,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(RateLimiterState {
                tokens: 100.0,
                max_tokens: 100.0,
                refill_rate: 0.83,
                last_refill: Instant::now(),
                requests_count: 0,
            }),
        }
    }

    pub fn wait_for_request(&self) {
        loop {
            if self.can_make_request() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    pub fn can_make_request(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        self.refill_tokens(&mut state);
        state.tokens >= 1.0
    }

    pub fn record_request(&self) {
        let mut state = self.state.lock().unwrap();
        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            state.requests_count += 1;
        }
    }

    pub fn get_remaining_tokens(&self) -> f64 {
        let mut state = self.state.lock().unwrap();
        self.refill_tokens(&mut state);
        state.tokens
    }

    fn refill_tokens(&self, state: &mut RateLimiterState) {
        let now = Instant::now();
        let time_passed = now.duration_since(state.last_refill).as_secs_f64();
        
        let tokens_to_add = time_passed * state.refill_rate;
        state.tokens = (state.max_tokens).min(state.tokens + tokens_to_add);
        state.last_refill = now;
    }
}

/// Simple in-memory cache with TTL
#[derive(Debug)]
pub struct Cache {
    entries: Mutex<HashMap<String, CacheEntry>>,
    max_size: usize,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    data: String,
    timestamp: SystemTime,
    ttl: Duration,
}

impl CacheEntry {
    pub fn is_expired(&self) -> bool {
        SystemTime::now()
            .duration_since(self.timestamp)
            .map(|duration| duration > self.ttl)
            .unwrap_or(true)
    }
}

impl Cache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            max_size: 1000,
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let mut entries = self.entries.lock().unwrap();
        if let Some(entry) = entries.get(key) {
            if !entry.is_expired() {
                return Some(entry.data.clone());
            } else {
                entries.remove(key);
            }
        }
        None
    }

    pub fn put(&self, key: String, data: String) {
        let mut entries = self.entries.lock().unwrap();
        
        if entries.len() >= self.max_size {
            self.evict_oldest(&mut entries);
        }

        let entry = CacheEntry {
            data,
            timestamp: SystemTime::now(),
            ttl: Duration::from_secs(300),
        };

        entries.insert(key, entry);
    }

    fn evict_oldest(&self, entries: &mut HashMap<String, CacheEntry>) {
        if let Some(key) = entries.keys().next().cloned() {
            entries.remove(&key);
        }
    }
}
