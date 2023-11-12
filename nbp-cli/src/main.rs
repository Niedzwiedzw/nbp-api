use clap::Parser;
#[allow(unused_imports)]
use eyre::{Result, WrapErr};
use nbp_api::{NbpApiClient, RatesRateEntry, RatesRequest, RatesResponse};
use rust_decimal::prelude::FromPrimitive;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// get the exchange rate
    ExchangeRate {
        /// value to be converted (defaults to 1.0)
        #[arg(long, short, default_value_t = rust_decimal::Decimal::from_usize(1).expect("valid decimal"))]
        value: rust_decimal::Decimal,
        /// source currency
        #[arg(long, short, default_value_t = RatesRequest::default().currency_code)]
        currency_code: String,
        /// defaults to yesterday
        #[arg(long, short, default_value_t = RatesRequest::default().date)]
        date: chrono::NaiveDate,
    },
}

fn setup_logging() {
    use tracing_subscriber::{prelude::*, EnvFilter};
    let subscriber = tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or(EnvFilter::try_from("info").unwrap()))
        .with(tracing_subscriber::fmt::Layer::new().with_writer(std::io::stderr));
    if let Err(message) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("logging setup failed: {message:?}");
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    setup_logging();
    let Cli { command } = Cli::parse();
    match command {
        Command::ExchangeRate {
            value,
            currency_code,
            date,
        } => {
            let client = NbpApiClient::new()?;
            let request = RatesRequest {
                date,
                currency_code,
                ..RatesRequest::default()
            };
            tracing::info!(?request, "fetching data");
            let RatesResponse {
                table: _,
                currency: _,
                code: _,
                rates,
            } = client.rates(request).await?;
            let &RatesRateEntry {
                no: _,
                effective_date: _,
                mid,
            } = &rates.first();
            let result = mid * value;
            println!("{result}");
            Ok(())
        }
    }
}
