pub mod network;
pub mod peer;
pub mod protocol;

use anyhow::{Result, Context};
use futures::StreamExt;
use libp2p::{
    core::upgrade,
    gossipsub::{self, IdentTopic, MessageAuthenticity},
    identify,
    identity::Keypair,
    kad::{self, store::MemoryStore},
    mdns,
    noise,
    ping,
    relay,
    dcutr,
    request_response::{self, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, PeerId, Swarm, Transport,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use protocol::{QuantraRequest, QuantraResponse};

// Define our custom network behaviour combining multiple protocols
#[derive(NetworkBehaviour)]
pub struct QuantraBehaviour {
    // Peer discovery via mDNS (local network)
    mdns: mdns::tokio::Behaviour,
    // DHT for peer discovery and content routing
    kademlia: kad::Behaviour<MemoryStore>,
    // Pub/sub messaging
    gossipsub: gossipsub::Behaviour,
    // Peer identification and version exchange
    identify: identify::Behaviour,
    // Connection keep-alive
    ping: ping::Behaviour,
    // Request/response protocol for direct messaging
    request_response: request_response::cbor::Behaviour<QuantraRequest, QuantraResponse>,
}

pub struct P2PNode {
    swarm: Swarm<QuantraBehaviour>,
    peer_id: PeerId,
    keypair: Keypair,
}

impl P2PNode {
    pub fn new() -> Result<Self> {
        // Generate identity keypair
        let local_key = Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        tracing::info!("Local peer id: {:?}", local_peer_id);

        // Create a keypair for message signing
        let message_id_fn = |message: &gossipsub::Message| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            gossipsub::MessageId::from(s.finish().to_string())
        };

        // Configure Gossipsub
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .message_id_fn(message_id_fn)
            .build()
            .map_err(|e| anyhow::anyhow!("Failed to build gossipsub config: {}", e))?;

        // Build the Gossipsub behaviour
        let gossipsub = gossipsub::Behaviour::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create gossipsub: {}", e))?;

        // Create Kademlia DHT
        let store = MemoryStore::new(local_peer_id);
        let kademlia = kad::Behaviour::new(local_peer_id, store);

        // Create mDNS for local peer discovery
        let mdns = mdns::tokio::Behaviour::new(
            mdns::Config::default(),
            local_peer_id,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create mDNS: {}", e))?;

        // Create identify protocol
        let identify = identify::Behaviour::new(
            identify::Config::new("/quantra/1.0.0".to_string(), local_key.public())
                .with_agent_version(format!("quantra-l/{}", env!("CARGO_PKG_VERSION"))),
        );

        // Create ping protocol
        let ping = ping::Behaviour::new(ping::Config::new());

        // Create request-response protocol
        let request_response = request_response::cbor::Behaviour::<QuantraRequest, QuantraResponse>::new(
            [(protocol::QUANTRA_PROTOCOL, ProtocolSupport::Full)],
            request_response::Config::default(),
        );

        // Combine all behaviours
        let behaviour = QuantraBehaviour {
            mdns,
            kademlia,
            gossipsub,
            identify,
            ping,
            request_response,
        };

        // Build the transport layer - simplified without relay for now
        let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(upgrade::Version::V1)
            .authenticate(
                noise::Config::new(&local_key)
                    .context("Failed to create noise config")?,
            )
            .multiplex(yamux::Config::default())
            .boxed();

        // Create the swarm
        let swarm = Swarm::new(
            transport,
            behaviour,
            local_peer_id,
            libp2p::swarm::Config::with_tokio_executor()
                .with_idle_connection_timeout(Duration::from_secs(60)),
        );

        Ok(Self {
            swarm,
            peer_id: local_peer_id,
            keypair: local_key,
        })
    }

    pub fn local_peer_id(&self) -> &PeerId {
        &self.peer_id
    }

    pub fn listen_on(&mut self, addr: &str) -> Result<()> {
        let multiaddr = addr
            .parse()
            .context("Failed to parse multiaddr")?;

        self.swarm
            .listen_on(multiaddr)
            .context("Failed to listen on address")?;

        tracing::info!("Listening on: {}", addr);
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("🚀 P2P node running with full networking!");
        tracing::info!("🔍 Peer discovery: mDNS (local) + Kademlia DHT (global)");
        tracing::info!("📡 Messaging: Gossipsub pub/sub");
        tracing::info!("🔒 Encryption: Noise Protocol (Ed25519)");
        tracing::info!("💡 Type 'help' for interactive commands");

        // Subscribe to default topic
        let topic = IdentTopic::new("quantra-default");
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic)
            .map_err(|e| anyhow::anyhow!("Failed to subscribe to topic: {}", e))?;

        tracing::info!("📢 Subscribed to topic: quantra-default");

        // Start listening for stdin commands (for interactive testing)
        let mut stdin = BufReader::new(tokio::io::stdin()).lines();

        loop {
            tokio::select! {
                // Handle swarm events
                event = self.swarm.select_next_some() => {
                    if let Err(e) = self.handle_event(event).await {
                        tracing::error!("Error handling event: {}", e);
                    }
                }

                // Handle stdin commands
                Ok(Some(line)) = stdin.next_line() => {
                    if let Err(e) = self.handle_command(&line as &str).await {
                        tracing::error!("Error handling command: {}", e);
                    }
                }
            }
        }
    }

    async fn handle_event(&mut self, event: SwarmEvent<QuantraBehaviourEvent>) -> Result<()> {
        match event {
            // Connection established
            SwarmEvent::ConnectionEstablished {
                peer_id,
                endpoint,
                num_established,
                ..
            } => {
                tracing::info!(
                    "✅ Connection established with peer: {} (endpoint: {}, total: {})",
                    peer_id,
                    endpoint.get_remote_address(),
                    num_established
                );
            }

            // Connection closed
            SwarmEvent::ConnectionClosed {
                peer_id,
                cause,
                num_established,
                ..
            } => {
                tracing::info!(
                    "❌ Connection closed with peer: {} (remaining: {}, cause: {:?})",
                    peer_id,
                    num_established,
                    cause
                );
            }

            // New listen address
            SwarmEvent::NewListenAddr { address, .. } => {
                tracing::info!("🎧 Listening on: {}", address);
            }

            // Behaviour events
            SwarmEvent::Behaviour(event) => {
                self.handle_behaviour_event(event).await?;
            }

            // Other events
            _ => {}
        }

        Ok(())
    }

    async fn handle_behaviour_event(&mut self, event: QuantraBehaviourEvent) -> Result<()> {
        match event {
            // mDNS discovered a peer
            QuantraBehaviourEvent::Mdns(mdns::Event::Discovered(peers)) => {
                for (peer_id, addr) in peers {
                    tracing::info!("🔍 mDNS discovered peer: {} at {}", peer_id, addr);
                    // Add peer to Kademlia DHT
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr);
                }
            }

            // mDNS peer expired
            QuantraBehaviourEvent::Mdns(mdns::Event::Expired(peers)) => {
                for (peer_id, addr) in peers {
                    tracing::info!("⏱️ mDNS peer expired: {} at {}", peer_id, addr);
                }
            }

            // Gossipsub message received
            QuantraBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                propagation_source,
                message_id,
                message,
            }) => {
                let msg_str = String::from_utf8_lossy(&message.data);
                tracing::info!(
                    "📨 Received message from {}: {} (id: {})",
                    propagation_source,
                    msg_str,
                    message_id
                );
            }

            // Identify protocol events
            QuantraBehaviourEvent::Identify(identify::Event::Received { peer_id, info, .. }) => {
                tracing::info!(
                    "🆔 Identified peer: {} - Agent: {}, Protocol: {}",
                    peer_id,
                    info.agent_version,
                    info.protocol_version
                );
                // Add all peer addresses to Kademlia
                for addr in info.listen_addrs {
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr);
                }
            }

            // Ping events
            QuantraBehaviourEvent::Ping(ping::Event {
                peer,
                result: Ok(rtt),
                ..
            }) => {
                tracing::debug!("🏓 Ping to {}: {:?}", peer, rtt);
            }

            // Kademlia events
            QuantraBehaviourEvent::Kademlia(kad::Event::RoutingUpdated {
                peer,
                addresses,
                ..
            }) => {
                tracing::info!("🗺️ Kademlia routing updated for {}: {:?}", peer, addresses);
            }

            // Request/Response events
            QuantraBehaviourEvent::RequestResponse(request_response::Event::Message {
                peer,
                message,
            }) => {
                match message {
                    request_response::Message::Request {
                        request, channel, ..
                    } => {
                        tracing::info!("📥 Request from {}: {:?}", peer, request);
                        // Handle request and send response
                        let response = self.handle_request(request).await?;
                        self.swarm
                            .behaviour_mut()
                            .request_response
                            .send_response(channel, response)
                            .map_err(|e| anyhow::anyhow!("Failed to send response: {:?}", e))?;
                    }
                    request_response::Message::Response { response, .. } => {
                        tracing::info!("📤 Response from {}: {:?}", peer, response);
                    }
                }
            }

            _ => {}
        }

        Ok(())
    }

    async fn handle_request(&self, request: QuantraRequest) -> Result<QuantraResponse> {
        match request {
            QuantraRequest::Ping => Ok(QuantraResponse::Pong),

            QuantraRequest::GetPeers => {
                let peers: Vec<String> = self
                    .swarm
                    .connected_peers()
                    .map(|p| p.to_string())
                    .collect();
                Ok(QuantraResponse::Peers(peers))
            }

            QuantraRequest::SendMessage { encrypted_data } => {
                tracing::info!("Received encrypted message: {} bytes", encrypted_data.len());
                Ok(QuantraResponse::MessageSent)
            }

            QuantraRequest::GetQuote { symbol } => {
                // Mock quote response
                Ok(QuantraResponse::Quote {
                    symbol,
                    price: 150.25,
                    timestamp: chrono::Utc::now().timestamp(),
                })
            }

            QuantraRequest::ProvisionESim { profile_data } => {
                tracing::info!("Provisioning eSIM: {} bytes", profile_data.len());
                Ok(QuantraResponse::ESimProvisioned {
                    activation_code: "LPA:1$sm-dp.example.com$activation-code".to_string(),
                })
            }
        }
    }

    async fn handle_command(&mut self, command: &str) -> Result<()> {
        let parts: Vec<&str> = command.trim().split_whitespace().collect();

        if parts.is_empty() {
            return Ok(());
        }

        match parts[0] {
            "peers" => {
                let peers: Vec<_> = self.swarm.connected_peers().collect();
                println!("📡 Connected peers ({}): {:?}", peers.len(), peers);
            }

            "msg" if parts.len() > 1 => {
                let message = parts[1..].join(" ");
                let topic = IdentTopic::new("quantra-default");
                self.swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(topic, message.as_bytes())
                    .map_err(|e| anyhow::anyhow!("Failed to publish: {}", e))?;
                println!("📤 Message published");
            }

            "dial" if parts.len() > 1 => {
                let addr: libp2p::Multiaddr = parts[1]
                    .parse()
                    .context("Invalid multiaddr")?;
                self.swarm.dial(addr)?;
                println!("📞 Dialing peer...");
            }

            "help" => {
                println!("Available commands:");
                println!("  peers       - List connected peers");
                println!("  msg <text>  - Broadcast message");
                println!("  dial <addr> - Connect to peer");
                println!("  help        - Show this help");
            }

            _ => {
                println!("Unknown command. Type 'help' for available commands.");
            }
        }

        Ok(())
    }
}
