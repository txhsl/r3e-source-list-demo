mod oracleerror;

use crate::oracleerror::OracleError;
use ethereum_types::U256;
use jsonpath_rust::JsonPathQuery;
use serde::Deserialize;
use serde_json::Value;

trait OracleSource {
    fn fetch(&self, params: Vec<u8>) -> Result<U256, OracleError>;
}

#[derive(Debug)]
pub struct TimeSourceAdapter {
    pub name: String,
}

impl TimeSourceAdapter {
    pub fn new(name: String) -> Self {
        TimeSourceAdapter { name }
    }
}

impl OracleSource for TimeSourceAdapter {
    fn fetch(&self, _params: Vec<u8>) -> Result<U256, OracleError> {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        Ok(U256::from(time))
    }
}

#[derive(Debug)]
pub struct RngSourceAdapter {
    pub name: String,
}

impl RngSourceAdapter {
    pub fn new(name: String) -> Self {
        RngSourceAdapter { name }
    }
}

impl OracleSource for RngSourceAdapter {
    fn fetch(&self, _params: Vec<u8>) -> Result<U256, OracleError> {
        //Ok(U256::from(new_stdrng().unwrap().gen::<u64>()))
        Ok(U256::from(1))
    }
}

#[derive(Debug, Deserialize)]
pub struct ExchangeSourceAdapter {
    pub name: String,
    pub url: String,
    pub params: Vec<String>,
    pub jsonpath: String,
    pub decimal: u32,
    pub bases: Vec<String>,
    pub quotes: Vec<String>,
}

impl ExchangeSourceAdapter {
    pub fn new(
        name: String,
        url: String,
        params: Vec<String>,
        jsonpath: String,
        decimal: u32,
        bases: Vec<String>,
        quotes: Vec<String>,
    ) -> Self {
        ExchangeSourceAdapter {
            name,
            url,
            params,
            jsonpath,
            decimal,
            bases,
            quotes,
        }
    }
}

impl OracleSource for ExchangeSourceAdapter {
    fn fetch(&self, params: Vec<u8>) -> Result<U256, OracleError> {
        let mut url = self.url.clone();
        for param in &self.params {
            url = url.replacen("{}", &param, 1);
        }
        url = url.replacen("{}", &self.quotes[params[0] as usize], 1);
        url = url.replacen("{}", &self.bases[params[1] as usize], 1);

        let response_text = reqwest::blocking::get(&url)?.text()?;
        let rpc_result: Value = serde_json::from_str(&response_text)?;
        let binding = rpc_result.path(&self.jsonpath).unwrap();
        let data = binding.get(0).ok_or(OracleError::DataNotFound)?;
        let price = match data.as_f64() {
            Some(val) => val,
            None => data.as_str().unwrap().parse::<f64>().unwrap(),
        };
        let price_adjusted = price * 10_f64.powf(self.decimal.into());
        Ok(ethereum_types::U256::from(price_adjusted as u64))
    }
}

#[derive(Debug)]
pub struct CustomSourceAdapter {
    pub url: String,
    pub jsonpath: String,
    pub decimal: u32,
}

impl CustomSourceAdapter {
    pub fn new(url: String, jsonpath: String, decimal: u32) -> Self {
        CustomSourceAdapter {
            url,
            jsonpath,
            decimal,
        }
    }
}

impl OracleSource for CustomSourceAdapter {
    fn fetch(&self, _params: Vec<u8>) -> Result<U256, OracleError> {
        let resp = reqwest::blocking::get(&self.url)?.text();
        if resp.is_err() {
            return Err(OracleError::Reqwest(resp.err().unwrap()));
        }

        let rpc_result: Value = serde_json::from_str(&resp.unwrap()).unwrap();
        let data = &rpc_result.path(&self.jsonpath).unwrap()[0];
        return if data.is_string() {
            let price =
                data.as_str().unwrap().parse::<f64>().unwrap() * 10_f64.powf(self.decimal.into());
            Ok(ethereum_types::U256::from(price as u64))
        } else {
            let price = data.as_f64().unwrap() * 10_f64.powf(self.decimal.into());
            Ok(ethereum_types::U256::from(price as u64))
        };
    }
}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_adapter_new() {
        let adapter = TimeSourceAdapter::new("time".to_string());
        assert_eq!(adapter.name, "time");
    }

    #[test]
    fn test_rng_adapter_new() {
        let adapter = RngSourceAdapter::new("rng".to_string());
        assert_eq!(adapter.name, "rng");
    }

    #[test]
    fn test_exchange_adapter_new() {
        let adapter = ExchangeSourceAdapter::new(
            "cryptocompare".to_string(),
            "https://min-api.cryptocompare.com/data/price?api_key={}&fsym={}&tsyms={}".to_string(),
            ["d4cf504725efe27b71ec7d213f5db583ef56e88cfbf437a3483d6bb43e9839ab".to_string()]
                .to_vec(),
            "$..*".to_string(),
            12,
            [
                "BTC".to_string(),
                "ETH".to_string(),
                "USDT".to_string(),
                "USDC".to_string(),
                "BNB".to_string(),
                "XRP".to_string(),
                "BUSD".to_string(),
            ]
            .to_vec(),
            [
                "BTC".to_string(),
                "ETH".to_string(),
                "USDT".to_string(),
                "USDC".to_string(),
                "BNB".to_string(),
                "XRP".to_string(),
                "BUSD".to_string(),
                "DOGE".to_string(),
                "ADA".to_string(),
                "MATIC".to_string(),
            ]
            .to_vec(),
        );

        assert_eq!(adapter.name, "cryptocompare");
    }

    #[test]
    fn test_custom_adapter_new() {
        let adapter = CustomSourceAdapter::new(
            "https://min-api.cryptocompare.com/data/price?api_key=d4cf504725efe27b71ec7d213f5db583ef56e88cfbf437a3483d6bb43e9839ab&fsym=BTC&tsyms=ETH".to_string(),
            "$..*".to_string(),
            12,
        );

        assert_eq!(adapter.decimal, 12);
    }

    #[test]
    fn test_time_adapter_fetch() {
        let adapter = TimeSourceAdapter::new("time".to_string());
        let result = adapter.fetch(vec![]);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_rng_adapter_fetch() {
        let adapter = RngSourceAdapter::new("rng".to_string());
        let result = adapter.fetch(vec![]);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_exchange_adapter_fetch() {
        let adapter = ExchangeSourceAdapter::new(
            "cryptocompare".to_string(),
            "https://min-api.cryptocompare.com/data/price?api_key={}&fsym={}&tsyms={}".to_string(),
            ["d4cf504725efe27b71ec7d213f5db583ef56e88cfbf437a3483d6bb43e9839ab".to_string()]
                .to_vec(),
            "$..*".to_string(),
            12,
            [
                "BTC".to_string(),
                "ETH".to_string(),
                "USDT".to_string(),
                "USDC".to_string(),
                "BNB".to_string(),
                "XRP".to_string(),
                "BUSD".to_string(),
            ]
            .to_vec(),
            [
                "BTC".to_string(),
                "ETH".to_string(),
                "USDT".to_string(),
                "USDC".to_string(),
                "BNB".to_string(),
                "XRP".to_string(),
                "BUSD".to_string(),
                "DOGE".to_string(),
                "ADA".to_string(),
                "MATIC".to_string(),
            ]
            .to_vec(),
        );
        let result = adapter.fetch([1, 2].to_vec());
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_custom_adapter_fetch() {
        let adapter = CustomSourceAdapter::new(
            "https://min-api.cryptocompare.com/data/price?api_key=d4cf504725efe27b71ec7d213f5db583ef56e88cfbf437a3483d6bb43e9839ab&fsym=BTC&tsyms=ETH".to_string(),
            "$..*".to_string(),
            12,
        );
        let result = adapter.fetch(vec![]);
        assert_eq!(result.is_ok(), true);
    }

    #[test]
    fn test_load_list() {
        let adapters: Vec<ExchangeSourceAdapter> = {
            let source_list = 
            r#"
            [
                {
                    "name": "cryptocompare",
                    "url": "https://min-api.cryptocompare.com/data/price?api_key={}&fsym={}&tsyms={}",
                    "params": ["d4cf504725efe27b71ec7d213f5db583ef56e88cfbf437a3483d6bb43e9839ab"],
                    "jsonpath": "$..*",
                    "decimal": 12,
                    "bases": ["USDT", "BTC", "ETH", "USDC", "BNB", "XRP", "BUSD"],
                    "quotes": ["BTC", "ETH", "USDT", "USDC", "BNB", "XRP", "BUSD", "DOGE", "ADA", "MATIC"]
                },
                {
                    "name": "binance",
                    "url": "https://data.binance.com/api/v3/ticker/price?symbol={}{}",
                    "params": [],
                    "jsonpath": "$.price",
                    "decimal": 12,
                    "bases": ["USDT"],
                    "quotes": ["BTC"]
                },
                {
                    "name": "mexc",
                    "url": "https://api.mexc.com/api/v3/avgPrice?symbol={}{}",
                    "params": [],
                    "jsonpath": "$.price",
                    "decimal": 12,
                    "bases": ["USDT"],
                    "quotes": ["BTC"]
                },
                {
                    "name": "hitbtc",
                    "url": "https://api.hitbtc.com/api/3/public/price/ticker/{}{}",
                    "params": [],
                    "jsonpath": "$.price",
                    "decimal": 12,
                    "bases": ["USDT"],
                    "quotes": ["BTC"]
                },
                {
                    "name": "kucoin",
                    "url": "https://openapi-sandbox.kucoin.com/api/v1/market/orderbook/level1?symbol={}-{}",
                    "params": [],
                    "jsonpath": "$.data.price",
                    "decimal": 12,
                    "bases": ["USDT"],
                    "quotes": ["BTC"]
                }
            ]
            "#
            .to_string();
            serde_json::from_str(&source_list).unwrap()
        };
        assert_eq!(adapters.len(), 5);
        for adapter in adapters {
            let result = adapter.fetch([0, 0].to_vec());
            assert_eq!(result.is_ok(), true);
        }
    }
}
