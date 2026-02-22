use crate::agent::a2a::types::AgentCard;
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Instant};

#[derive(Clone)]
pub struct AgentDiscovery {
    cache: Arc<RwLock<DiscoveryCache>>,
    static_agents: HashMap<String, AgentCard>,
    http_discovery_endpoints: Vec<String>,
    cache_ttl: Duration,
}

#[derive(Clone)]
struct DiscoveryCache {
    agents: HashMap<String, AgentCard>,
    cached_at: Option<Instant>,
}

impl DiscoveryCache {
    fn new() -> Self {
        Self {
            agents: HashMap::new(),
            cached_at: None,
        }
    }

    fn is_fresh(&self, ttl: Duration) -> bool {
        match self.cached_at {
            Some(cached_at) => cached_at.elapsed() < ttl,
            None => false,
        }
    }
}

impl AgentDiscovery {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(DiscoveryCache::new())),
            static_agents: HashMap::new(),
            http_discovery_endpoints: Vec::new(),
            cache_ttl: Duration::from_secs(300),
        }
    }

    pub fn with_static_agents(mut self, agents: HashMap<String, AgentCard>) -> Self {
        self.static_agents = agents;
        self
    }

    pub fn with_http_discovery_endpoints(mut self, endpoints: Vec<String>) -> Self {
        self.http_discovery_endpoints = endpoints;
        self
    }

    pub fn with_cache_ttl(mut self, ttl_secs: u64) -> Self {
        self.cache_ttl = Duration::from_secs(ttl_secs);
        self
    }

    pub async fn discover_agent(&self, agent_id: &str) -> Result<Option<AgentCard>> {
        if let Some(card) = self.static_agents.get(agent_id) {
            return Ok(Some(card.clone()));
        }

        let cache = self.cache.read().await;
        if cache.is_fresh(self.cache_ttl) {
            if let Some(card) = cache.agents.get(agent_id) {
                return Ok(Some(card.clone()));
            }
        }
        drop(cache);

        self.refresh_cache().await?;

        let cache = self.cache.read().await;
        Ok(cache.agents.get(agent_id).cloned())
    }

    pub async fn discover_all(&self) -> Result<Vec<AgentCard>> {
        let mut all_agents: Vec<AgentCard> = self.static_agents.values().cloned().collect();

        for endpoint in &self.http_discovery_endpoints {
            if let Ok(agents) = self.fetch_from_endpoint(endpoint).await {
                all_agents.extend(agents);
            }
        }

        Ok(all_agents)
    }

    pub async fn refresh_cache(&self) -> Result<()> {
        let mut cache = self.cache.write().await;
        let mut discovered_agents: HashMap<String, AgentCard> = self.static_agents.clone();

        for endpoint in &self.http_discovery_endpoints {
            if let Ok(agents) = self.fetch_from_endpoint(endpoint).await {
                for agent in agents {
                    discovered_agents.insert(agent.name.clone(), agent);
                }
            }
        }

        *cache = DiscoveryCache {
            agents: discovered_agents,
            cached_at: Some(Instant::now()),
        };

        Ok(())
    }

    async fn fetch_from_endpoint(&self, endpoint: &str) -> Result<Vec<AgentCard>> {
        let client = reqwest::Client::new();
        let response = client
            .get(endpoint)
            .timeout(Duration::from_secs(10))
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("Discovery endpoint returned status: {}", response.status());
        }

        let agents: Vec<AgentCard> = response.json().await?;
        Ok(agents)
    }

    pub async fn get_cached(&self) -> Vec<AgentCard> {
        let cache = self.cache.read().await;
        cache.agents.values().cloned().collect()
    }

    pub fn load_from_config_file(path: &PathBuf) -> Result<HashMap<String, AgentCard>> {
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let content = std::fs::read_to_string(path)?;
        let agents: Vec<AgentCard> = serde_json::from_str(&content)?;

        let mut map = HashMap::new();
        for agent in agents {
            map.insert(agent.name.clone(), agent);
        }

        Ok(map)
    }

    pub fn save_to_config_file(path: &PathBuf, agents: &[AgentCard]) -> Result<()> {
        let content = serde_json::to_string_pretty(agents)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Default for AgentDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

pub struct StaticDiscovery {
    agents: HashMap<String, AgentCard>,
}

impl StaticDiscovery {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>, card: AgentCard) -> Self {
        self.agents.insert(agent_id.into(), card);
        self
    }

    pub fn get(&self, agent_id: &str) -> Option<&AgentCard> {
        self.agents.get(agent_id)
    }

    pub fn all(&self) -> Vec<&AgentCard> {
        self.agents.values().collect()
    }
}

impl Default for StaticDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HttpDiscovery {
    endpoints: Vec<String>,
    cache: Arc<RwLock<HashMap<String, AgentCard>>>,
    cache_ttl: Duration,
}

impl HttpDiscovery {
    pub fn new() -> Self {
        Self {
            endpoints: Vec::new(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(300),
        }
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoints.push(endpoint.into());
        self
    }

    pub fn with_cache_ttl(mut self, ttl_secs: u64) -> Self {
        self.cache_ttl = Duration::from_secs(ttl_secs);
        self
    }

    pub async fn discover(&self, agent_id: &str) -> Result<Option<AgentCard>> {
        for endpoint in &self.endpoints {
            let url = format!(
                "{}/.well-known/agent-card.json",
                endpoint.trim_end_matches('/')
            );

            let client = reqwest::Client::new();
            match client.get(&url).timeout(self.cache_ttl).send().await {
                Ok(response) if response.status().is_success() => {
                    if let Ok(card) = response.json::<AgentCard>().await {
                        if card.name == agent_id {
                            let mut cache = self.cache.write().await;
                            cache.insert(agent_id.to_string(), card.clone());
                            return Ok(Some(card));
                        }
                    }
                }
                _ => {}
            }
        }

        let cache = self.cache.read().await;
        Ok(cache.get(agent_id).cloned())
    }

    pub async fn discover_all(&self) -> Result<Vec<AgentCard>> {
        let mut all_agents = Vec::new();

        for endpoint in &self.endpoints {
            let url = format!(
                "{}/.well-known/agent-card.json",
                endpoint.trim_end_matches('/')
            );

            let client = reqwest::Client::new();
            match client.get(&url).timeout(self.cache_ttl).send().await {
                Ok(response) if response.status().is_success() => {
                    if let Ok(card) = response.json::<AgentCard>().await {
                        all_agents.push(card);
                    }
                }
                _ => {}
            }
        }

        Ok(all_agents)
    }
}

impl Default for HttpDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DiscoveryManager {
    static_discovery: StaticDiscovery,
    http_discovery: HttpDiscovery,
    cache: Arc<RwLock<DiscoveryCache>>,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        Self {
            static_discovery: StaticDiscovery::new(),
            http_discovery: HttpDiscovery::new(),
            cache: Arc::new(RwLock::new(DiscoveryCache::new())),
        }
    }

    pub fn with_static_agent(mut self, agent_id: impl Into<String>, card: AgentCard) -> Self {
        self.static_discovery = self.static_discovery.with_agent(agent_id, card);
        self
    }

    pub fn with_http_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.http_discovery = self.http_discovery.with_endpoint(endpoint);
        self
    }

    pub async fn find_agent(&self, agent_id: &str) -> Result<Option<AgentCard>> {
        if let Some(card) = self.static_discovery.get(agent_id) {
            return Ok(Some(card.clone()));
        }

        self.http_discovery.discover(agent_id).await
    }

    pub async fn find_all_agents(&self) -> Result<Vec<AgentCard>> {
        let mut agents: Vec<AgentCard> = self.static_discovery.all().into_iter().cloned().collect();

        let http_agents = self.http_discovery.discover_all().await?;
        agents.extend(http_agents);

        Ok(agents)
    }

    pub fn expose_agent_card(&self, card: AgentCard) -> AgentCard {
        card
    }
}

impl Default for DiscoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_discovery_basic() {
        let discovery = StaticDiscovery::new()
            .with_agent(
                "agent-1",
                AgentCard::new("Agent 1", "1.0.0", "http://localhost:8000"),
            )
            .with_agent(
                "agent-2",
                AgentCard::new("Agent 2", "1.0.0", "http://localhost:8001"),
            );

        assert!(discovery.get("agent-1").is_some());
        assert!(discovery.get("agent-2").is_some());
        assert!(discovery.get("agent-3").is_none());
        assert_eq!(discovery.all().len(), 2);
    }

    #[test]
    fn agent_discovery_load_from_file_not_found() {
        let path = PathBuf::from("/nonexistent/path.json");
        let agents = AgentDiscovery::load_from_config_file(&path).unwrap();
        assert!(agents.is_empty());
    }

    #[tokio::test]
    async fn discovery_manager_static_agent() {
        let manager = DiscoveryManager::new()
            .with_static_agent("test-agent", AgentCard::new("Test", "1.0.0", "http://test"));

        let agent = manager.find_agent("test-agent").await.unwrap();
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().name, "Test");
    }

    #[tokio::test]
    async fn discovery_manager_not_found() {
        let manager = DiscoveryManager::new();

        let agent = manager.find_agent("nonexistent").await.unwrap();
        assert!(agent.is_none());
    }
}
