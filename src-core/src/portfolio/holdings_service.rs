use crate::account::account_service::AccountService;
use crate::activity::activity_service::ActivityService;
use crate::asset::asset_service::AssetService;
use crate::fx::fx_service::CurrencyExchangeService;
use crate::models::{Holding, Performance};
use diesel::r2d2::{ConnectionManager, Pool};
use diesel::SqliteConnection;
use std::collections::{HashMap, HashSet};

pub struct HoldingsService {
    account_service: AccountService,
    activity_service: ActivityService,
    asset_service: AssetService,
    fx_service: CurrencyExchangeService,
    base_currency: String,
    pool: Pool<ConnectionManager<SqliteConnection>>,
}

impl HoldingsService {
    pub fn new(pool: Pool<ConnectionManager<SqliteConnection>>, base_currency: String) -> Self {
        HoldingsService {
            account_service: AccountService::new(pool.clone()),
            activity_service: ActivityService::new(pool.clone()),
            asset_service: AssetService::new(pool.clone()),
            fx_service: CurrencyExchangeService::new(pool.clone()),
            base_currency,
            pool,
        }
    }

    pub fn compute_holdings(&self) -> Result<Vec<Holding>, Box<dyn std::error::Error>> {
        let mut holdings: HashMap<String, Holding> = HashMap::new();
        let accounts = self.account_service.get_accounts()?;
        let activities = self.activity_service.get_trading_activities()?;
        let assets = self.asset_service.get_assets()?;

        for activity in activities {
            //find asset by id
            let asset = match assets.iter().find(|a| a.id == activity.asset_id) {
                Some(found_asset) => found_asset,
                None => {
                    println!("Asset not found for id: {}", activity.asset_id);
                    continue; // Skip this iteration if the asset is not found
                }
            };

            //find account by id
            let account = accounts
                .iter()
                .find(|a| a.id == activity.account_id)
                .unwrap();

            let key = format!("{}-{}", activity.account_id, activity.asset_id);
            let holding = holdings.entry(key.clone()).or_insert_with(|| Holding {
                id: key,
                symbol: activity.asset_id.clone(),
                symbol_name: asset.name.clone(),
                holding_type: asset.asset_type.clone().unwrap_or_default(),
                quantity: 0.0,
                currency: activity.currency.clone(),
                base_currency: "CAD".to_string(),
                market_price: None,          // You need to provide market price
                average_cost: None,          // Will be calculated
                market_value: 0.0,           // Will be calculated
                book_value: 0.0,             // Will be calculated
                market_value_converted: 0.0, // Will be calculated
                book_value_converted: 0.0,   // Will be calculated
                performance: Performance {
                    total_gain_percent: 0.0,
                    total_gain_amount: 0.0,
                    total_gain_amount_converted: 0.0,
                    day_gain_percent: Some(0.0),
                    day_gain_amount: Some(0.0),
                    day_gain_amount_converted: Some(0.0),
                },
                account: Some(account.clone()),
                asset_class: asset.asset_class.clone(),
                asset_sub_class: asset.asset_sub_class.clone(),
                sectors: asset
                    .sectors
                    .clone()
                    .map(|s| serde_json::from_str(&s).unwrap_or_default()),
            });

            match activity.activity_type.as_str() {
                "BUY" => {
                    holding.quantity += activity.quantity;
                    holding.book_value += activity.quantity * activity.unit_price + activity.fee;
                }
                "SELL" => {
                    holding.quantity -= activity.quantity;
                    holding.book_value -= activity.quantity * activity.unit_price + activity.fee;
                }
                _ => {}
            }
        }

        // Collect all unique symbols from holdings
        let unique_symbols: HashSet<String> = holdings
            .values()
            .map(|holding| holding.symbol.clone())
            .collect();

        let symbols: Vec<String> = unique_symbols.into_iter().collect();

        // Fetch quotes for each symbol asynchronously
        let mut quotes = HashMap::new();
        for symbol in symbols {
            match self.asset_service.get_latest_quote(&symbol) {
                Ok(quote) => {
                    quotes.insert(symbol, quote);
                }
                Err(e) => {
                    println!("Error fetching quote for symbol {}: {}", symbol, e);
                    // Handle the error as per your logic, e.g., continue, return an error, etc.
                }
            }
        }

        // Post-processing for each holding
        for holding in holdings.values_mut() {
            if let Some(quote) = quotes.get(&holding.symbol) {
                //prinln!("Quote: {:?}", quote);
                holding.market_price = Some(quote.close); // Assuming you want to use the 'close' value as market price
            }
            holding.average_cost = Some(holding.book_value / holding.quantity);
            holding.market_value = holding.quantity * holding.market_price.unwrap_or(0.0);
            holding.market_value_converted = self.fx_service.convert_currency(
                holding.market_value,
                &holding.currency,
                &self.base_currency,
            )?;
            holding.book_value_converted = self.fx_service.convert_currency(
                holding.book_value,
                &holding.currency,
                &self.base_currency,
            )?;

            // Calculate performance metrics
            holding.performance.total_gain_amount = holding.market_value - holding.book_value;
            holding.performance.total_gain_percent = if holding.book_value != 0.0 {
                holding.performance.total_gain_amount / holding.book_value * 100.0
            } else {
                0.0
            };
            holding.performance.total_gain_amount_converted = self.fx_service.convert_currency(
                holding.performance.total_gain_amount,
                &holding.currency,
                &self.base_currency,
            )?;
        }

        holdings
            .into_values()
            .filter(|holding| holding.quantity > 0.0)
            .map(Ok)
            .collect::<Result<Vec<_>, _>>()
    }
}
