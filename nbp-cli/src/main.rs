use std::{future::ready, pin::pin};

use clap::Parser;
#[allow(unused_imports)]
use eyre::{Result, WrapErr};
use futures::StreamExt;
use nbp_api::{NbpApiClient, RatesRateEntry, RatesRequest, RatesResponse};
use rust_decimal::prelude::FromPrimitive;
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Default, Debug, clap::ValueEnum, Clone)]
enum OutputMode {
    #[default]
    JustValue,
    Details,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(long, default_value = "just-value")]
    output_mode: OutputMode,
}

#[derive(clap::Subcommand)]
enum Command {
    /// get the exchange rate
    ExchangeRate {
        /// value to be converted (defaults to 1.0)
        #[arg(long, default_value_t = rust_decimal::Decimal::from_usize(1).expect("valid decimal"))]
        value: rust_decimal::Decimal,
        /// source currency
        #[arg(long, default_value_t = RatesRequest::default().currency_code)]
        currency_code: String,
        /// defaults to today
        #[arg(long, default_value_t = RatesRequest::default().date)]
        date: chrono::NaiveDate,
        /// it is client's job to find latest entry, as nbp api returns 404 for non-banking days
        #[arg(long, default_value_t = 7)]
        max_day_before_checks: i64,
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
    let Cli {
        command,
        output_mode,
    } = Cli::parse();
    match command {
        Command::ExchangeRate {
            value,
            currency_code,
            date,
            max_day_before_checks,
        } => {
            let client = NbpApiClient::new()?;
            let base_request = RatesRequest {
                date,
                currency_code: currency_code.clone(),
                ..RatesRequest::default()
            };
            info!(?base_request, "performing request");
            let request_for_date = move |date| RatesRequest {
                date,
                ..base_request.clone()
            };
            let RatesResponse {
                table,
                currency,
                code,
                rates,
            } = {
                let dates = (0..max_day_before_checks)
                    .map(|offset| date - chrono::Duration::days(offset))
                    .collect::<Vec<_>>();
                let mut stream = futures::stream::iter(dates.clone())
                    .map(request_for_date)
                    .then(|rates_request| client.rates(rates_request))
                    .filter_map(|res| ready(res.ok()));
                let mut stream = pin!(stream);
                stream.next().await.ok_or_else(|| {
                    eyre::eyre!("could not find any entry in following dates: {dates:?}")
                })?
            };
            let &RatesRateEntry {
                no,
                effective_date,
                mid,
            } = &rates.first();
            let result = mid * value;
            info!(%table, %currency, %code, %no, %effective_date, %mid, "found rating for specified parameters");
            match output_mode {
                OutputMode::JustValue => println!("{result}"),
                OutputMode::Details => {
                    println!("1 {code} = {mid} PLN ({effective_date}, tab. {table}, {no})")
                }
            }

            Ok(())
        }
    }
}
