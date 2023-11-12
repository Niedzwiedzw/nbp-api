use clap::Parser;
#[allow(unused_imports)]
use eyre::{Result, WrapErr};
use kalkulator_walutowy_api::{NbpApiClient, RatesRateEntry, RatesRequest, RatesResponse};
use rust_decimal::prelude::FromPrimitive;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, short, default_value_t = chrono::Local::now().date_naive())]
    date: chrono::NaiveDate,
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// get the exchange rate
    ExchangeRate {
        #[arg(long, short, default_value_t = rust_decimal::Decimal::from_usize(1).expect("valid decimal"))]
        value: rust_decimal::Decimal,
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
    let Cli { date, command } = Cli::parse();
    match command {
        Command::ExchangeRate { value } => {
            let client = NbpApiClient::new()?;
            let RatesResponse {
                table,
                currency,
                code,
                rates,
            } = client
                .rates(RatesRequest {
                    date,
                    ..RatesRequest::default()
                })
                .await?;
            let &RatesRateEntry {
                no,
                effective_date,
                mid,
            } = &rates.first();
            let result = mid * value;
            println!("{result}");
            Ok(())
        }
    }
    // info!("it works");
    // client.rates(RatesRequest {
    //     table_name: todo!(),
    //     currency_code: todo!(),
    //     date: todo!(),
    // });
}
