#!/bin/bash
# QuantraBand Zero-Trust Parallel VM Failover Watchdog
# Monitors and auto-restarts failed nodes

BINARY="./target/release/quantraband"
PORTS=(9000 9001 9002)
LOG_DIR="/home/worm/.quantra/logs"
mkdir -p "$LOG_DIR"

echo "🔒 QuantraBand Zero-Trust Failover Watchdog"
echo "==========================================="
echo "Monitoring ${#PORTS[@]} parallel Zero-Trust nodes..."
echo ""

# Function to check if a node is running on a port
check_node() {
    local port=$1
    pgrep -f "quantraband p2p --listen.*:$port" > /dev/null
    return $?
}

# Function to start a node
start_node() {
    local port=$1
    echo "$(date '+%Y-%m-%d %H:%M:%S') 🚀 Starting node on port $port..."
    $BINARY p2p --listen "/ip4/0.0.0.0/tcp/$port" --zero-trust >> "$LOG_DIR/node_$port.log" 2>&1 &
    echo "$(date '+%Y-%m-%d %H:%M:%S') ✅ Node $port started with PID $!"
}

# Initial status check
echo "Initial node status:"
for port in "${PORTS[@]}"; do
    if check_node $port; then
        echo "  ✅ Node $port: RUNNING"
    else
        echo "  ❌ Node $port: DOWN - Starting..."
        start_node $port
    fi
done
echo ""

# Failover loop
echo "Starting failover monitoring (Ctrl+C to stop)..."
while true; do
    for port in "${PORTS[@]}"; do
        if ! check_node $port; then
            echo "$(date '+%Y-%m-%d %H:%M:%S') ⚠️  FAILOVER: Node $port DOWN - Restarting..."
            start_node $port
            sleep 2
        fi
    done
    sleep 5
done
