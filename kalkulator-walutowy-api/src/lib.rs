#[allow(unused_imports)]
use eyre::{Result, WrapErr};
use futures_util::TryFutureExt;
use nonempty::NonEmpty;
use reqwest::ClientBuilder;
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Clone)]
pub struct NbpApiClient {
    client: reqwest::Client,
}

impl NbpApiClient {
    #[instrument(err)]
    pub fn new() -> Result<Self> {
        ClientBuilder::new()
            .build()
            .wrap_err("building kalkulator walutowy client")
            .map(|client| Self { client })
    }
}

/// {
///   "table": "A",
///   "currency": "euro",
///   "code": "EUR",
///   "rates": [
///     {
///       "no": "218/A/NBP/2023",
///       "effectiveDate": "2023-11-10",
///       "mid": 4.4227
///     }
///   ]
/// }
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RatesResponse {
    pub table: String,
    pub currency: String,
    pub code: String,
    pub rates: NonEmpty<RatesRateEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RatesRateEntry {
    pub no: String,
    pub effective_date: String,
    pub mid: rust_decimal::Decimal,
}

/// ❯ curl http://api.nbp.pl/api/exchangerates/rates/A/EUR/2023-11-10
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RatesRequest {
    pub table_name: String,
    pub currency_code: String,
    pub date: chrono::NaiveDate,
}

impl std::default::Default for RatesRequest {
    fn default() -> Self {
        Self {
            table_name: "A".into(),
            currency_code: "EUR".into(),
            date: chrono::Local::now().date_naive(),
        }
    }
}

pub trait ErrExt {
    fn wrap_err(self, message: &'static str) -> eyre::Error;
}

impl<E> ErrExt for E
where
    E: Into<eyre::Error>,
{
    fn wrap_err(self, message: &'static str) -> eyre::Error {
        Result::<(), eyre::Error>::Err(self.into())
            .wrap_err(message)
            .expect_err("this is an error")
    }
}

impl NbpApiClient {
    #[instrument(level = "debug", ret, err)]
    pub async fn rates(
        &self,
        RatesRequest {
            table_name,
            currency_code,
            date,
        }: RatesRequest,
    ) -> Result<RatesResponse> {
        let date = date.format("%Y-%m-%d");
        self.client
            .get(format!(
                "http://api.nbp.pl/api/exchangerates/rates/{table_name}/{currency_code}/{date}"
            ))
            .send()
            .map_err(|e| e.wrap_err("sending request"))
            .and_then(|response| {
                response
                    .text()
                    .map_err(|e| e.wrap_err("reading response text"))
                    .and_then(|text| {
                        std::future::ready(
                            serde_json::from_str(&text)
                                .wrap_err_with(|| format!("bad response: {text}")),
                        )
                    })
            })
            .await
    }
}
