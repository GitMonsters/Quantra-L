# Quantra-L Profiler - Quick Start Guide

This guide will help you get started with the Google Cloud Profiler integration for Quantra-L.

## Prerequisites

- Quantra-L P2P VPN installed and running
- Google Cloud Platform account
- `gcloud` CLI installed and authenticated
- Rust 1.70+ and Cargo
- Python 3.11+ (for backend)
- Node.js 18+ (for dashboard)

## Phase 1: Basic Setup (30 minutes)

### Step 1: Enable Google Cloud APIs

```bash
# Set your GCP project ID
export GCP_PROJECT="your-project-id"
gcloud config set project $GCP_PROJECT

# Enable required APIs
gcloud services enable cloudprofiler.googleapis.com
gcloud services enable storage.googleapis.com
gcloud services enable cloudfunctions.googleapis.com
gcloud services enable pubsub.googleapis.com
gcloud services enable compute.googleapis.com
```

### Step 2: Create GCS Bucket for Profiles

```bash
# Create bucket (globally unique name)
gsutil mb -c STANDARD -l US gs://quantra-profiles-${GCP_PROJECT}/

# Set lifecycle policy (delete profiles older than 30 days)
cat > lifecycle.json <<EOF
{
  "lifecycle": {
    "rule": [
      {
        "action": {"type": "Delete"},
        "condition": {"age": 30}
      }
    ]
  }
}
EOF

gsutil lifecycle set lifecycle.json gs://quantra-profiles-${GCP_PROJECT}/
```

### Step 3: Create Service Account

```bash
# Create service account
gcloud iam service-accounts create quantra-profiler \
  --display-name="Quantra Profiler Service Account"

# Grant permissions
gcloud projects add-iam-policy-binding $GCP_PROJECT \
  --member="serviceAccount:quantra-profiler@${GCP_PROJECT}.iam.gserviceaccount.com" \
  --role="roles/storage.objectCreator"

gcloud projects add-iam-policy-binding $GCP_PROJECT \
  --member="serviceAccount:quantra-profiler@${GCP_PROJECT}.iam.gserviceaccount.com" \
  --role="roles/cloudprofiler.agent"

# Download key
gcloud iam service-accounts keys create ~/quantra-profiler-key.json \
  --iam-account=quantra-profiler@${GCP_PROJECT}.iam.gserviceaccount.com

export GOOGLE_APPLICATION_CREDENTIALS=~/quantra-profiler-key.json
```

### Step 4: Add Profiler to Quantra-L

```bash
cd /home/worm/quantra

# Add dependencies to Cargo.toml
cat >> Cargo.toml <<EOF

# Profiler dependencies
pprof = { version = "0.13", features = ["flamegraph", "protobuf-codec"] }
google-cloud-storage = "0.17"
prost = "0.12"
EOF

# Create profiler module
mkdir -p src/profiler
```

**Create `src/profiler/mod.rs`:**

```rust
pub mod agent;
pub mod config;
pub mod exporter;
pub mod metrics;

pub use agent::ProfilerAgent;
pub use config::ProfilerConfig;
```

**Create `src/profiler/config.rs`:**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilerConfig {
    /// Enable profiling (default: false)
    pub enabled: bool,

    /// Sample rate in Hz (default: 100 = every 10ms)
    pub sample_rate: u32,

    /// Upload interval in seconds (default: 60)
    pub upload_interval_secs: u64,

    /// GCS bucket for storing profiles
    pub gcs_bucket: String,

    /// GCP project ID
    pub gcp_project: String,
}

impl Default for ProfilerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sample_rate: 100,
            upload_interval_secs: 60,
            gcs_bucket: std::env::var("GCS_BUCKET")
                .unwrap_or_else(|_| "quantra-profiles".to_string()),
            gcp_project: std::env::var("GCP_PROJECT")
                .unwrap_or_else(|_| "quantra-project".to_string()),
        }
    }
}

impl ProfilerConfig {
    pub fn from_env() -> Self {
        Self {
            enabled: std::env::var("PROFILER_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            sample_rate: std::env::var("PROFILER_SAMPLE_RATE")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .unwrap_or(100),
            upload_interval_secs: std::env::var("PROFILER_UPLOAD_INTERVAL")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            gcs_bucket: std::env::var("GCS_BUCKET")
                .unwrap_or_else(|_| "quantra-profiles".to_string()),
            gcp_project: std::env::var("GCP_PROJECT")
                .unwrap_or_else(|_| "quantra-project".to_string()),
        }
    }
}
```

**Create `src/profiler/agent.rs`:**

```rust
use anyhow::{Context, Result};
use pprof::ProfilerGuard;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use super::config::ProfilerConfig;
use super::exporter::GcsExporter;

pub struct ProfilerAgent {
    config: ProfilerConfig,
    guard: Arc<RwLock<Option<ProfilerGuard<'static>>>>,
    exporter: Arc<GcsExporter>,
}

impl ProfilerAgent {
    pub fn new(config: ProfilerConfig) -> Result<Self> {
        let exporter = Arc::new(GcsExporter::new(&config)?);

        Ok(Self {
            config,
            guard: Arc::new(RwLock::new(None)),
            exporter,
        })
    }

    /// Start profiling with automatic uploads
    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            log::info!("Profiler disabled in config");
            return Ok(());
        }

        log::info!(
            "Starting profiler: sample_rate={}Hz, upload_interval={}s",
            self.config.sample_rate,
            self.config.upload_interval_secs
        );

        // Start CPU profiling
        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(self.config.sample_rate as i32)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .context("Failed to start profiler")?;

        *self.guard.write().await = Some(guard);

        // Spawn upload task
        let guard_clone = Arc::clone(&self.guard);
        let exporter_clone = Arc::clone(&self.exporter);
        let upload_interval = self.config.upload_interval_secs;

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(upload_interval));

            loop {
                ticker.tick().await;

                if let Err(e) = Self::collect_and_upload(&guard_clone, &exporter_clone).await {
                    log::error!("Failed to collect/upload profile: {}", e);
                }
            }
        });

        log::info!("Profiler started successfully");
        Ok(())
    }

    async fn collect_and_upload(
        guard: &Arc<RwLock<Option<ProfilerGuard<'static>>>>,
        exporter: &Arc<GcsExporter>,
    ) -> Result<()> {
        // Build profile report
        let report = {
            let guard_lock = guard.read().await;
            if let Some(ref g) = *guard_lock {
                g.report().build().context("Failed to build profile report")?
            } else {
                anyhow::bail!("Profiler not started");
            }
        };

        // Serialize to pprof protobuf format
        let mut buffer = Vec::new();
        report.pprof().context("Failed to convert to pprof")?.write_to_vec(&mut buffer)
            .context("Failed to serialize pprof")?;

        log::info!("Collected profile: {} bytes", buffer.len());

        // Upload to GCS
        let profile_id = format!("profile-{}.pb", chrono::Utc::now().timestamp());
        exporter.upload(&profile_id, buffer).await
            .context("Failed to upload profile to GCS")?;

        log::info!("Uploaded profile: {}", profile_id);

        Ok(())
    }

    /// Stop profiling
    pub async fn stop(&self) -> Result<()> {
        let mut guard = self.guard.write().await;
        *guard = None;
        log::info!("Profiler stopped");
        Ok(())
    }
}
```

**Create `src/profiler/exporter.rs`:**

```rust
use anyhow::{Context, Result};
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::upload::{UploadObjectRequest, UploadType};

use super::config::ProfilerConfig;

pub struct GcsExporter {
    client: Client,
    bucket: String,
}

impl GcsExporter {
    pub fn new(config: &ProfilerConfig) -> Result<Self> {
        let client_config = ClientConfig::default()
            .with_auth()
            .context("Failed to create GCS client config")?;

        let client = Client::new(client_config);

        Ok(Self {
            client,
            bucket: config.gcs_bucket.clone(),
        })
    }

    pub async fn upload(&self, profile_id: &str, data: Vec<u8>) -> Result<()> {
        let upload_type = UploadType::Simple(data.into());

        self.client
            .upload_object(
                &UploadObjectRequest {
                    bucket: self.bucket.clone(),
                    ..Default::default()
                },
                upload_type,
                &format!("profiles/{}", profile_id),
            )
            .await
            .context("GCS upload failed")?;

        Ok(())
    }
}
```

**Create `src/profiler/metrics.rs`:**

```rust
use prometheus::{IntCounter, Histogram, Gauge, register_int_counter, register_histogram, register_gauge};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref PROFILES_COLLECTED: IntCounter =
        register_int_counter!(
            "quantra_profiles_collected_total",
            "Total number of profiles collected"
        ).unwrap();

    pub static ref PROFILE_UPLOAD_DURATION: Histogram =
        register_histogram!(
            "quantra_profile_upload_duration_seconds",
            "Duration of profile uploads to GCS"
        ).unwrap();

    pub static ref PROFILER_OVERHEAD: Gauge =
        register_gauge!(
            "quantra_profiler_cpu_overhead_percent",
            "Estimated CPU overhead of profiler"
        ).unwrap();

    pub static ref PROFILE_SIZE_BYTES: Histogram =
        register_histogram!(
            "quantra_profile_size_bytes",
            "Size of collected profiles in bytes"
        ).unwrap();
}
```

### Step 5: Integrate with Main Application

**Update `src/main.rs`:**

```rust
mod profiler;  // Add this line

use profiler::{ProfilerAgent, ProfilerConfig};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Initialize profiler
    let profiler_config = ProfilerConfig::from_env();
    if profiler_config.enabled {
        let profiler = ProfilerAgent::new(profiler_config)?;
        profiler.start().await?;
        log::info!("Profiler initialized");
    }

    // Rest of your application...
    let cli = Cli::parse();
    match cli.command {
        // ... existing commands
    }

    Ok(())
}
```

### Step 6: Build and Test

```bash
# Build with profiler
cargo build --release

# Run with profiler enabled
PROFILER_ENABLED=true \
PROFILER_SAMPLE_RATE=100 \
PROFILER_UPLOAD_INTERVAL=60 \
GCS_BUCKET=quantra-profiles-${GCP_PROJECT} \
GCP_PROJECT=${GCP_PROJECT} \
GOOGLE_APPLICATION_CREDENTIALS=~/quantra-profiler-key.json \
./target/release/quantra-l p2p --listen /ip4/0.0.0.0/tcp/9000

# In another terminal, generate load
for i in {1..1000}; do
  curl -X POST http://localhost:9000/api/test/message
done

# Wait 60 seconds, then check GCS bucket
gsutil ls gs://quantra-profiles-${GCP_PROJECT}/profiles/

# Download and inspect profile
gsutil cp gs://quantra-profiles-${GCP_PROJECT}/profiles/profile-*.pb ./
```

## Phase 2: Analysis Backend (1 hour)

### Step 1: Set Up Python Environment

```bash
cd /home/worm/quantra
mkdir -p profiler-backend
cd profiler-backend

# Create virtual environment
python3 -m venv venv
source venv/bin/activate

# Install dependencies
pip install \
  google-cloud-profiler==4.0.0 \
  google-cloud-storage==2.10.0 \
  pprof-parser==0.1.0 \
  pandas==2.1.0 \
  fastapi==0.104.0 \
  uvicorn==0.24.0 \
  plotly==5.18.0 \
  scikit-learn==1.3.0
```

### Step 2: Create Simple Flame Graph Generator

**Create `profiler-backend/analyzer/flame_graph.py`:**

```python
#!/usr/bin/env python3
import sys
from google.cloud import storage
from pprof_parser import parse_pprof

def generate_flame_graph(gcs_path: str):
    """
    Download pprof from GCS and generate flame graph.

    Args:
        gcs_path: gs://bucket/path/to/profile.pb
    """
    # Parse GCS path
    parts = gcs_path.replace("gs://", "").split("/", 1)
    bucket_name = parts[0]
    blob_path = parts[1]

    # Download profile
    client = storage.Client()
    bucket = client.bucket(bucket_name)
    blob = bucket.blob(blob_path)
    profile_data = blob.download_as_bytes()

    # Parse pprof
    profile = parse_pprof(profile_data)

    # Build call stack tree
    stacks = []
    for sample in profile.samples:
        stack = []
        for location_id in sample.location_ids:
            location = profile.locations[location_id]
            for line in location.lines:
                function = profile.functions[line.function_id]
                stack.append(function.name)

        stacks.append({
            "stack": ";".join(reversed(stack)),
            "value": sample.value[0]  # CPU samples
        })

    # Generate flame graph format (collapsed stacks)
    collapsed = {}
    for s in stacks:
        key = s["stack"]
        collapsed[key] = collapsed.get(key, 0) + s["value"]

    # Output
    for stack, count in sorted(collapsed.items(), key=lambda x: x[1], reverse=True):
        print(f"{stack} {count}")

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python flame_graph.py gs://bucket/path/to/profile.pb")
        sys.exit(1)

    generate_flame_graph(sys.argv[1])
```

### Step 3: Test Flame Graph Generation

```bash
# Run analyzer on latest profile
LATEST=$(gsutil ls gs://quantra-profiles-${GCP_PROJECT}/profiles/ | tail -1)
python analyzer/flame_graph.py $LATEST > /tmp/flamegraph.txt

# View top 10 hotspots
head -10 /tmp/flamegraph.txt
```

**Expected output:**
```
main;p2p::run;gossipsub::handle_message 1250
main;p2p::run;kademlia::lookup 875
main;zerotrust::verify_identity;crypto::ed25519::verify 623
main;p2p::run;yamux::poll 412
...
```

## Phase 3: Dashboard (Optional - 2 hours)

### Quick Dashboard with Plotly

**Create `profiler-backend/dashboard.py`:**

```python
#!/usr/bin/env python3
import plotly.graph_objects as go
from fastapi import FastAPI
from fastapi.responses import HTMLResponse
import subprocess

app = FastAPI()

@app.get("/", response_class=HTMLResponse)
async def dashboard():
    # Get latest profile
    result = subprocess.run(
        ["gsutil", "ls", "gs://quantra-profiles-*/profiles/"],
        capture_output=True,
        text=True
    )
    latest = result.stdout.strip().split("\n")[-1]

    # Generate flame graph data
    result = subprocess.run(
        ["python", "analyzer/flame_graph.py", latest],
        capture_output=True,
        text=True
    )

    # Parse collapsed stacks
    stacks = []
    for line in result.stdout.strip().split("\n")[:20]:  # Top 20
        stack, count = line.rsplit(" ", 1)
        stacks.append({"stack": stack, "count": int(count)})

    # Create bar chart
    fig = go.Figure(data=[
        go.Bar(
            x=[s["count"] for s in stacks],
            y=[s["stack"].split(";")[-1] for s in stacks],  # Just function name
            orientation='h'
        )
    ])

    fig.update_layout(
        title="Quantra-L CPU Hotspots",
        xaxis_title="CPU Samples",
        yaxis_title="Function",
        height=800
    )

    return fig.to_html()

if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="0.0.0.0", port=8000)
```

**Run dashboard:**

```bash
python dashboard.py
# Open http://localhost:8000 in browser
```

## Quick Testing Checklist

- [ ] GCS bucket created and accessible
- [ ] Service account has correct permissions
- [ ] Quantra-L builds with profiler dependencies
- [ ] Profiler starts without errors (check logs)
- [ ] Profiles appear in GCS bucket after 60 seconds
- [ ] Flame graph generator parses profiles successfully
- [ ] Dashboard shows CPU hotspots

## Troubleshooting

### Issue: Profiler crashes with "permission denied"

```bash
# Check service account permissions
gcloud projects get-iam-policy $GCP_PROJECT \
  --flatten="bindings[].members" \
  --filter="bindings.members:serviceAccount:quantra-profiler@*"

# Should show roles/storage.objectCreator and roles/cloudprofiler.agent
```

### Issue: No profiles in GCS bucket

```bash
# Check environment variables
env | grep -E 'PROFILER|GCS|GCP'

# Check logs
tail -f /var/log/quantra-l.log | grep -i profiler

# Test GCS upload manually
echo "test" | gsutil cp - gs://quantra-profiles-${GCP_PROJECT}/test.txt
```

### Issue: High CPU overhead (>2%)

```bash
# Reduce sample rate
PROFILER_SAMPLE_RATE=50 ./target/release/quantra-l p2p

# Or increase upload interval
PROFILER_UPLOAD_INTERVAL=300 ./target/release/quantra-l p2p
```

## Next Steps

After completing the quick start:

1. **Review architecture:** Read `PROFILER_ARCHITECTURE.md`
2. **Implement Phase 2:** Real-time analysis pipeline
3. **Set up monitoring:** Prometheus metrics for profiler health
4. **Enable auto-scaling:** Implement optimization engine

## Cost Estimation

For a development setup (1 node, 8 hours/day):

- Google Cloud Profiler: $0 (free tier)
- Cloud Storage: ~$0.05/month (few hundred MB)
- **Total: ~$0.05/month**

## Support

- **Architecture docs:** `PROFILER_ARCHITECTURE.md`
- **GitHub issues:** https://github.com/GitMonsters/Quantra-L/issues
- **Quantra-L Discord:** [Link to Discord server]

---

**Last Updated:** November 25, 2025
**Tested With:** Quantra-L v0.1.0, Google Cloud SDK 455.0.0
