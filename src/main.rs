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
        Commands::ProvisionEsim { carrier, plan } => {
            info!("Provisioning eSIM for carrier: {}, plan: {}", carrier, plan);
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
            println!("eSIM Profile provisioned!");
            println!("ICCID: {}", profile.iccid);
            println!("Activation Code: {}", profile.activation_code);
            println!("\nGenerating QR code...");
            let qr_data = esim_manager.generate_qr_code(&profile).await?;
            println!("QR code generated: {} bytes", qr_data.len());
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
    }

    Ok(())
}
