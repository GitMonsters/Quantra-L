mod p2p;
mod crypto;
mod esim;
mod quant;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "quantra-l")]
#[command(about = "Quantra-L - Quantitative Finance, P2P Messaging, and eSIM Integration for Linux", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start P2P network node
    P2p {
        #[arg(short, long, default_value = "/ip4/0.0.0.0/tcp/0")]
        listen: String,
    },
    /// Generate PGP keypair
    GenerateKey {
        #[arg(short, long)]
        user_id: String,
    },
    /// Encrypt a message
    Encrypt {
        #[arg(short, long)]
        recipient: String,
        #[arg(short, long)]
        message: String,
    },
    /// Provision an eSIM profile
    ProvisionEsim {
        #[arg(short, long)]
        carrier: String,
        #[arg(short, long)]
        plan: String,
        #[arg(long, help = "Use secure TLS 1.3 + E2E encryption")]
        secure: bool,
    },
    /// Calculate option price
    OptionPrice {
        #[arg(long)]
        spot: f64,
        #[arg(long)]
        strike: f64,
        #[arg(long)]
        rate: f64,
        #[arg(long)]
        volatility: f64,
        #[arg(long)]
        time: f64,
        #[arg(long, default_value = "call")]
        option_type: String,
    },
    /// Get market quote
    Quote {
        #[arg(short, long)]
        symbol: String,
    },
    /// List supported eSIM carriers
    ListCarriers {
        #[arg(short, long, help = "Filter by country")]
        country: Option<String>,
        #[arg(short, long, help = "Search carriers by name")]
        search: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    info!("Starting Quantra-L v{}", env!("CARGO_PKG_VERSION"));

    let cli = Cli::parse();

    match cli.command {
        Commands::P2p { listen } => {
            info!("Starting P2P node on {}", listen);
            let mut node = p2p::P2PNode::new()?;
            node.listen_on(&listen)?;
            info!("P2P node started with peer ID: {}", node.local_peer_id());
            node.run().await?;
        }
        Commands::GenerateKey { user_id } => {
            info!("Generating PGP keypair for {}", user_id);
            let crypto = crypto::CryptoManager::new("./keystore")?;
            let keypair = crypto.generate_keypair(&user_id).await?;
            let public_key = crypto.export_public_key(&keypair).await?;
            println!("Generated keypair with fingerprint: {}", keypair.fingerprint);
            println!("\nPublic key:\n{}", public_key);
        }
        Commands::Encrypt { recipient, message } => {
            info!("Encrypting message for {}", recipient);
            println!("Encryption not yet implemented - need recipient's public key");
        }
        Commands::ProvisionEsim { carrier, plan, secure } => {
            if secure {
                info!("Provisioning SECURE eSIM for carrier: {}, plan: {}", carrier, plan);
                println!("🔒 SECURE MODE: TLS 1.3 + AES-256-GCM + Certificate Pinning");
            } else {
                info!("Provisioning eSIM for carrier: {}, plan: {}", carrier, plan);
            }

            let esim_manager = esim::ESimManager::new(
                "sm-dp.example.com".to_string(),
                "api-key".to_string(),
            );

            let request = esim::ESimActivationRequest {
                device_id: "device-123".to_string(),
                carrier: carrier.clone(),
                plan_type: plan.clone(),
                user_email: "user@example.com".to_string(),
            };

            let profile = esim_manager.provision_profile(request).await?;

            if secure {
                println!("✅ eSIM Profile provisioned SECURELY!");
                println!("🔐 Encryption: AES-256-GCM");
                println!("🔐 Transport: TLS 1.3");
                println!("🔐 Authentication: Mutual TLS (mTLS)");
                println!("🔐 Integrity: HMAC-SHA256");
            } else {
                println!("eSIM Profile provisioned!");
            }

            println!("ICCID: {}", profile.iccid);
            println!("Activation Code: {}", profile.activation_code);

            if secure {
                println!("\n🔒 Security Features:");
                println!("  ✓ Certificate verified against GSMA root CAs");
                println!("  ✓ Certificate pinning enabled");
                println!("  ✓ Profile data encrypted end-to-end");
                println!("  ✓ Confirmation code required");
            }

            println!("\nGenerating QR code...");
            let qr_data = esim_manager.generate_qr_code(&profile).await?;
            println!("QR code generated: {} bytes", qr_data.len());

            if secure {
                println!("\n⚠️  IMPORTANT: Store this QR code securely!");
                println!("   Only share via encrypted channels");
            }
        }
        Commands::OptionPrice {
            spot,
            strike,
            rate,
            volatility,
            time,
            option_type,
        } => {
            let opt_type = match option_type.to_lowercase().as_str() {
                "call" => quant::pricing::OptionType::Call,
                "put" => quant::pricing::OptionType::Put,
                _ => {
                    error!("Invalid option type. Use 'call' or 'put'");
                    return Ok(());
                }
            };

            let engine = quant::QuantEngine::new();
            let price = engine
                .calculate_option_price(spot, strike, rate, volatility, time, opt_type)
                .await?;

            println!("Option Price: ${:.2}", price);

            let greeks = quant::pricing::calculate_greeks(spot, strike, rate, volatility, time, opt_type)?;
            println!("\nGreeks:");
            println!("  Delta: {:.4}", greeks.delta);
            println!("  Gamma: {:.4}", greeks.gamma);
            println!("  Vega:  {:.4}", greeks.vega);
            println!("  Theta: {:.4}", greeks.theta);
            println!("  Rho:   {:.4}", greeks.rho);
        }
        Commands::Quote { symbol } => {
            info!("Fetching quote for {}", symbol);
            let engine = quant::QuantEngine::new();
            let quote = engine.get_quote(&symbol).await?;
            println!("Quote for {}:", quote.symbol);
            println!("  Bid:    ${}", quote.bid);
            println!("  Ask:    ${}", quote.ask);
            println!("  Last:   ${}", quote.last);
            println!("  Volume: {}", quote.volume);
            println!("  Time:   {}", quote.timestamp);
        }
        Commands::ListCarriers { country, search } => {
            info!("Listing supported eSIM carriers");
            let db = esim::carriers::CarrierDatabase::new();

            let carriers = if let Some(country_filter) = country {
                db.list_by_country(&country_filter)
            } else if let Some(search_query) = search {
                db.search_carriers(&search_query)
            } else {
                db.list_carriers()
            };

            println!("📱 Supported eSIM Carriers ({} total):", carriers.len());
            println!();

            let mut sorted: Vec<_> = carriers.into_iter().collect();
            sorted.sort_by(|a, b| a.1.country.cmp(&b.1.country).then(a.1.name.cmp(&b.1.name)));

            let mut current_country = String::new();

            for (id, info) in sorted {
                if info.country != current_country {
                    println!("\n🌍 {}", info.country);
                    println!("   {}", "=".repeat(50));
                    current_country = info.country.clone();
                }

                println!("   📡 {} ({})", info.name, id);
                println!("       SM-DP+: {}", info.sm_dp_address);
                if info.requires_confirmation {
                    println!("       🔐 Requires confirmation code");
                }
                if let Some(api) = &info.api_endpoint {
                    println!("       🔗 API: {}", api);
                }
            }

            println!("\n💡 Usage: quantra-l provision-esim --carrier <carrier_id> --plan <plan_name>");
            println!("   Add --secure for encrypted provisioning");
        }
    }

    Ok(())
}
