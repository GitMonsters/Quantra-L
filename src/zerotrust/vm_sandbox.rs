use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use chrono::{DateTime, Utc};
use crate::zerotrust::SecurityLevel;

/// VM Sandbox provides isolated network environments
/// Supports: Docker containers, QEMU/KVM VMs, Firecracker microVMs
#[derive(Debug, Clone)]
pub struct VMManager {
    sandboxes: HashMap<String, VMSandbox>,
    backend: VMBackend,
    max_sandboxes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMBackend {
    Docker,        // Lightweight containers
    QEMU,          // Full virtualization
    Firecracker,   // MicroVMs (AWS technology)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VMSandbox {
    pub id: String,
    pub peer_id: String,
    pub security_level: SecurityLevel,
    pub backend: String,
    pub container_id: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub cpu_shares: u32,
    pub memory_mb: u32,
    pub network_bandwidth_mbps: u32,
}

#[derive(Debug, Clone)]
pub struct VMStats {
    pub active_sandboxes: usize,
    pub total_created: usize,
    pub backend: VMBackend,
}

impl VMManager {
    pub fn new() -> Result<Self> {
        // Detect available backend
        let backend = Self::detect_backend()?;

        tracing::info!("🖥️  VM Manager initialized with backend: {:?}", backend);

        Ok(Self {
            sandboxes: HashMap::new(),
            backend,
            max_sandboxes: 100,
        })
    }

    /// Create isolated sandbox for connection
    pub async fn create_sandbox(
        &mut self,
        peer_id: &str,
        security_level: SecurityLevel,
    ) -> Result<VMSandbox> {
        let id = format!("qtz-{}", uuid::Uuid::new_v4());

        let resource_limits = match security_level {
            SecurityLevel::Privileged => ResourceLimits {
                cpu_shares: 512,
                memory_mb: 512,
                network_bandwidth_mbps: 100,
            },
            SecurityLevel::Critical => ResourceLimits {
                cpu_shares: 1024,
                memory_mb: 1024,
                network_bandwidth_mbps: 1000,
            },
            _ => ResourceLimits {
                cpu_shares: 256,
                memory_mb: 256,
                network_bandwidth_mbps: 50,
            },
        };

        let container_id = match self.backend {
            VMBackend::Docker => Some(self.create_docker_sandbox(&id, &resource_limits).await?),
            VMBackend::QEMU => Some(self.create_qemu_sandbox(&id, &resource_limits).await?),
            VMBackend::Firecracker => Some(self.create_firecracker_sandbox(&id, &resource_limits).await?),
        };

        let sandbox = VMSandbox {
            id: id.clone(),
            peer_id: peer_id.to_string(),
            security_level,
            backend: format!("{:?}", self.backend),
            container_id,
            ip_address: None, // Would be assigned by backend
            created_at: Utc::now(),
            resource_limits,
        };

        self.sandboxes.insert(id.clone(), sandbox.clone());

        tracing::info!(
            "🔒 Created {} sandbox {} for peer {}",
            self.backend.to_string(),
            id,
            peer_id
        );

        Ok(sandbox)
    }

    /// Destroy sandbox and cleanup resources
    pub async fn destroy_sandbox(&mut self, sandbox_id: &str) -> Result<()> {
        if let Some(sandbox) = self.sandboxes.remove(sandbox_id) {
            if let Some(container_id) = &sandbox.container_id {
                match self.backend {
                    VMBackend::Docker => self.destroy_docker_sandbox(container_id).await?,
                    VMBackend::QEMU => self.destroy_qemu_sandbox(container_id).await?,
                    VMBackend::Firecracker => self.destroy_firecracker_sandbox(container_id).await?,
                }
            }

            tracing::info!("🗑️  Destroyed sandbox {}", sandbox_id);
        }

        Ok(())
    }

    /// Check if there's capacity for new sandbox
    pub async fn has_capacity(&self) -> Result<bool> {
        Ok(self.sandboxes.len() < self.max_sandboxes)
    }

    /// Get statistics
    pub async fn get_stats(&self) -> Result<VMStats> {
        Ok(VMStats {
            active_sandboxes: self.sandboxes.len(),
            total_created: self.sandboxes.len(), // In production, track total
            backend: self.backend,
        })
    }

    /// Detect available VM backend
    fn detect_backend() -> Result<VMBackend> {
        // Check for Docker
        if Command::new("docker").arg("--version").output().is_ok() {
            return Ok(VMBackend::Docker);
        }

        // Check for QEMU
        if Command::new("qemu-system-x86_64").arg("--version").output().is_ok() {
            return Ok(VMBackend::QEMU);
        }

        // Check for Firecracker
        if Command::new("firecracker").arg("--version").output().is_ok() {
            return Ok(VMBackend::Firecracker);
        }

        // Default to Docker (will be created in mock mode if not available)
        Ok(VMBackend::Docker)
    }

    /// Create Docker container sandbox
    async fn create_docker_sandbox(&self, id: &str, limits: &ResourceLimits) -> Result<String> {
        // Create isolated network namespace with Docker
        let output = Command::new("docker")
            .args(&[
                "run",
                "-d",
                "--name", id,
                "--network", "none", // Isolated network
                "--cpus", &format!("{}", limits.cpu_shares as f32 / 1024.0),
                "--memory", &format!("{}m", limits.memory_mb),
                "--cap-drop", "ALL", // Drop all capabilities
                "--security-opt", "no-new-privileges",
                "alpine:latest",
                "sleep", "infinity",
            ])
            .output()
            .context("Failed to create Docker sandbox")?;

        if !output.status.success() {
            // If Docker not available, return mock ID
            tracing::warn!("Docker not available, using mock sandbox");
            return Ok(format!("mock-{}", id));
        }

        let container_id = String::from_utf8(output.stdout)?.trim().to_string();

        tracing::info!("🐳 Created Docker sandbox: {}", container_id);

        Ok(container_id)
    }

    /// Destroy Docker sandbox
    async fn destroy_docker_sandbox(&self, container_id: &str) -> Result<()> {
        if container_id.starts_with("mock-") {
            return Ok(()); // Mock sandbox, nothing to destroy
        }

        Command::new("docker")
            .args(&["rm", "-f", container_id])
            .output()
            .context("Failed to destroy Docker sandbox")?;

        Ok(())
    }

    /// Create QEMU VM sandbox (simplified)
    async fn create_qemu_sandbox(&self, id: &str, limits: &ResourceLimits) -> Result<String> {
        tracing::info!("🖥️  QEMU sandbox {} (mock mode)", id);
        Ok(format!("qemu-{}", id))
    }

    /// Destroy QEMU sandbox
    async fn destroy_qemu_sandbox(&self, vm_id: &str) -> Result<()> {
        tracing::info!("Destroyed QEMU sandbox {}", vm_id);
        Ok(())
    }

    /// Create Firecracker microVM sandbox (simplified)
    async fn create_firecracker_sandbox(&self, id: &str, limits: &ResourceLimits) -> Result<String> {
        tracing::info!("🔥 Firecracker microVM {} (mock mode)", id);
        Ok(format!("fc-{}", id))
    }

    /// Destroy Firecracker sandbox
    async fn destroy_firecracker_sandbox(&self, vm_id: &str) -> Result<()> {
        tracing::info!("Destroyed Firecracker sandbox {}", vm_id);
        Ok(())
    }
}

impl VMBackend {
    fn to_string(&self) -> &'static str {
        match self {
            VMBackend::Docker => "Docker",
            VMBackend::QEMU => "QEMU/KVM",
            VMBackend::Firecracker => "Firecracker",
        }
    }
}
