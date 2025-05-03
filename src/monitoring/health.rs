use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};

/// Health check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// System is healthy
    Healthy,
    
    /// System is degraded but functional
    Degraded,
    
    /// System is unhealthy
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

/// Health check component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthComponent {
    /// Component name
    pub name: String,
    
    /// Component status
    pub status: HealthStatus,
    
    /// Additional details
    pub details: String,
    
    /// Last check time (seconds since epoch)
    pub last_check: u64,
}

/// Health check manager
pub struct HealthManager {
    /// Components being monitored
    components: RwLock<HashMap<String, HealthComponent>>,
    
    /// Overall system health
    system_health: RwLock<HealthStatus>,
    
    /// Last update time
    last_update: RwLock<Instant>,
}

impl HealthManager {
    /// Create a new health manager
    pub fn new() -> Self {
        Self {
            components: RwLock::new(HashMap::new()),
            system_health: RwLock::new(HealthStatus::Healthy),
            last_update: RwLock::new(Instant::now()),
        }
    }
    
    /// Register a component for health checks
    pub fn register_component(&self, name: &str) {
        let mut components = self.components.write().unwrap();
        components.insert(name.to_string(), HealthComponent {
            name: name.to_string(),
            status: HealthStatus::Healthy,
            details: "Initialized".to_string(),
            last_check: current_time_secs(),
        });
    }
    
    /// Update component health
    pub fn update_component_health(
        &self,
        name: &str,
        status: HealthStatus,
        details: &str,
    ) {
        let mut components = self.components.write().unwrap();
        
        if let Some(component) = components.get_mut(name) {
            component.status = status;
            component.details = details.to_string();
            component.last_check = current_time_secs();
        } else {
            // Component not registered, add it
            components.insert(name.to_string(), HealthComponent {
                name: name.to_string(),
                status,
                details: details.to_string(),
                last_check: current_time_secs(),
            });
        }
        
        // Update system health
        self.update_system_health();
        
        // Update last update time
        let mut last_update = self.last_update.write().unwrap();
        *last_update = Instant::now();
    }
    
    /// Update overall system health based on components
    fn update_system_health(&self) {
        let components = self.components.read().unwrap();
        let mut system_health = self.system_health.write().unwrap();
        
        // Start with healthy
        let mut overall = HealthStatus::Healthy;
        
        // Check all components
        for component in components.values() {
            match component.status {
                HealthStatus::Unhealthy => {
                    // Any unhealthy component makes the system unhealthy
                    overall = HealthStatus::Unhealthy;
                    break;
                }
                HealthStatus::Degraded => {
                    // Any degraded component makes the system degraded (unless already unhealthy)
                    if overall == HealthStatus::Healthy {
                        overall = HealthStatus::Degraded;
                    }
                }
                HealthStatus::Healthy => {
                    // Healthy components don't change the overall status
                }
            }
        }
        
        *system_health = overall;
    }
    
    /// Get overall system health
    pub fn system_health(&self) -> HealthStatus {
        *self.system_health.read().unwrap()
    }
    
    /// Get all component health
    pub fn component_health(&self) -> Vec<HealthComponent> {
        let components = self.components.read().unwrap();
        components.values().cloned().collect()
    }
    
    /// Get health of a specific component
    pub fn get_component_health(&self, name: &str) -> Option<HealthComponent> {
        let components = self.components.read().unwrap();
        components.get(name).cloned()
    }
    
    /// Get last update time
    pub fn last_update(&self) -> Duration {
        let last_update = self.last_update.read().unwrap();
        last_update.elapsed()
    }
    
    /// Run health checks on all registered components
    pub fn run_health_checks(&self) {
        // In a real implementation, you would actually check the health of each component
        // For this example, we'll just update a few example components
        
        // Check database health
        self.update_component_health(
            "database", 
            HealthStatus::Healthy,
            "Connected, 0ms latency",
        );
        
        // Check consensus health
        self.update_component_health(
            "consensus", 
            HealthStatus::Healthy,
            "Operating normally",
        );
        
        // Check network health
        let peer_count = 10; // Example value
        if peer_count < 5 {
            self.update_component_health(
                "network", 
                HealthStatus::Degraded,
                &format!("Low peer count: {}", peer_count),
            );
        } else {
            self.update_component_health(
                "network", 
                HealthStatus::Healthy,
                &format!("Connected peers: {}", peer_count),
            );
        }
        
        // Check sync status
        let is_synced = true; // Example value
        if is_synced {
            self.update_component_health(
                "sync", 
                HealthStatus::Healthy,
                "Fully synced",
            );
        } else {
            self.update_component_health(
                "sync", 
                HealthStatus::Degraded,
                "Still syncing",
            );
        }
        
        // Check memory usage
        let memory_usage_mb = 500; // Example value in MB
        let memory_limit_mb = 1000; // Example limit in MB
        
        if memory_usage_mb > memory_limit_mb {
            self.update_component_health(
                "memory", 
                HealthStatus::Unhealthy,
                &format!("Memory usage too high: {}MB > {}MB", memory_usage_mb, memory_limit_mb),
            );
        } else if memory_usage_mb > memory_limit_mb * 8 / 10 {
            self.update_component_health(
                "memory", 
                HealthStatus::Degraded,
                &format!("Memory usage high: {}MB", memory_usage_mb),
            );
        } else {
            self.update_component_health(
                "memory", 
                HealthStatus::Healthy,
                &format!("Memory usage: {}MB", memory_usage_mb),
            );
        }
    }
}

/// Get current time in seconds since epoch
fn current_time_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_health_manager() {
        let health_manager = HealthManager::new();
        
        // Register components
        health_manager.register_component("database");
        health_manager.register_component("network");
        
        // Initially all should be healthy
        assert_eq!(health_manager.system_health(), HealthStatus::Healthy);
        
        // Update component to degraded
        health_manager.update_component_health(
            "network",
            HealthStatus::Degraded,
            "Low peer count",
        );
        
        // System should now be degraded
        assert_eq!(health_manager.system_health(), HealthStatus::Degraded);
        
        // Update component to unhealthy
        health_manager.update_component_health(
            "database",
            HealthStatus::Unhealthy,
            "Connection failed",
        );
        
        // System should now be unhealthy
        assert_eq!(health_manager.system_health(), HealthStatus::Unhealthy);
        
        // Check component health
        let db_health = health_manager.get_component_health("database").unwrap();
        assert_eq!(db_health.status, HealthStatus::Unhealthy);
        assert_eq!(db_health.details, "Connection failed");
        
        // Fix all components
        health_manager.update_component_health(
            "database",
            HealthStatus::Healthy,
            "Connected",
        );
        
        health_manager.update_component_health(
            "network",
            HealthStatus::Healthy,
            "10 peers",
        );
        
        // System should be healthy again
        assert_eq!(health_manager.system_health(), HealthStatus::Healthy);
    }
} 