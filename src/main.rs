use std::io::Read;
use std::str::FromStr;
use std::thread;
use std::time::{Duration, SystemTime};

use json::{JsonValue, parse};
use tungstenite::connect;
use url::Url;

fn get_amount(data: &JsonValue, needed_data: &str) -> f64 {
    let mut sum_value: f64 = 0.0;

    for item in data.members() {
        let index = match needed_data {
            "amount" => 1,
            "value" => 0,
            _ => 0
        };

        let amount = f64::from_str(&*item[index].clone().to_string()).unwrap();
        sum_value += amount;
    }
    sum_value
}

fn socket_price() {
    let (mut socket_depth, _) = connect(
        Url::parse("wss://stream.binance.com:9443/ws/btcusdt@trade").unwrap()
    ).expect("Can't connect");
    println!("{:?}", socket_depth.read_message().expect("Error reading message"));
    loop {
        let msg = socket_depth.read_message().expect("Error reading message");
        let data = parse(&*msg.to_string()).unwrap();
        println!("{:?}", &*data["p"].to_string());
    }
}

fn get_api_price(price_type: &str) -> f64 {
    let url = match price_type {
        "current" => "https://api2.binance.com/api/v3/ticker/price?symbol=BTCUSDT",
        "average" => "https://api2.binance.com/api/v3/avgPrice?symbol=BTCUSDT",
        _ => "127.0.0.1:8888"
    };
    let mut res = reqwest::blocking::get(url).unwrap();
    let mut body = String::new();
    res.read_to_string(&mut body).expect("panic message");
    let json_body = parse(&*body.clone()).unwrap();
    f64::from_str(&*json_body["price"].to_string()).unwrap()
}

fn get_price(data: &JsonValue, needed_data: &str) -> f64 {
    match needed_data {
        "bid" => {
            let mut minimal_value = 99999999.00;
            for item in data.members() {
                let amount = f64::from_str(&*item[0].clone().to_string()).unwrap();
                minimal_value = {
                    if amount <= minimal_value {
                        amount
                    } else {
                        minimal_value
                    }
                };
            };
            minimal_value
        },
        "ask" => {
            let mut max_value = 0.00;
            for item in data.members() {
                let amount = f64::from_str(&*item[0].clone().to_string()).unwrap();
                max_value = {
                    if amount >= max_value {
                        amount
                    } else {
                        max_value
                    }
                };
            };
            max_value
        }
        _ => {
            0.00
        }
    }
}

fn process() {
    let mut last_status = "NEW";
    let mut price_bought = 0.00;
    let mut price_sold: f64 = 0.00;
    let mut profit = 100.00;
    let mut lock = false;
    let mut avg_price: f64 = get_api_price("average");
    let mut time_start = SystemTime::now();
    let raw_time_start = SystemTime::now();
    let (mut socket_depth, _) = connect(
        Url::parse("wss://stream.binance.com:9443/ws/btcusdt@depth@100ms").unwrap()
    ).expect("Can't connect");
    loop {
        let msg = socket_depth.read_message().expect("Error reading message");
        let data = parse(&*msg.to_string()).unwrap();
        let ask_array = data["a"].clone();
        let ask_amount = get_amount(&ask_array, "amount");
        let bid_array = data["b"].clone();
        let bid_amount = get_amount(&bid_array, "amount");
        let status = {
            if ask_amount - bid_amount < 0.0 {
                "DOWN"
            } else if ask_amount - bid_amount == 0.0 {
                "EVEN"
            } else {
                "UP"
            }
        };
        let mut body: f64 = 0.00;
        if SystemTime::now().duration_since(time_start).unwrap().ge(&Duration::new(60, 0)) {
            if lock {
                body = get_api_price("current");
                avg_price = (avg_price + body) / 2.0;
                println!("{body}  {price_bought}");
                if body > price_bought {
                    price_sold = body;
                    profit += profit * ((price_sold - price_bought) / price_sold);
                    lock = false;
                    println!("Profit is {:?}", profit);
                    time_start = SystemTime::now();
                    // sell assets
                }
            }
        }
        if status != last_status {
            match status {
                "DOWN" => {
                    if lock {
                        body = get_api_price("current");
                        avg_price = (avg_price + body) / 2.0;
                        println!("{body}  {price_bought}");
                        if body > price_bought {
                            price_sold = body;
                            profit += profit * ((price_sold - price_bought) / price_sold);
                            lock = false;
                            println!("Profit is {:?}", profit);
                            time_start = SystemTime::now();
                            // sell assets
                        }
                    }
                },
                "UP" => {
                    if !lock || last_status == "NEW" {
                        body = get_api_price("current");
                        avg_price = (avg_price + body) / 2.0;
                        if body < avg_price {
                            price_bought = body;
                            lock = true
                            // buy assets
                        }
                    }
                },
                _ => {
                    body = get_api_price("current");
                    avg_price = (avg_price + body) / 2.0;
                }
            }
        }
        last_status = status;
        println!("{status}");
        println!("{:?}", body);
        println!("AVERAGE PRICE {:?}", avg_price);
        println!("{:?}", SystemTime::now().duration_since(raw_time_start).unwrap());
        println!("-------------------------------------------")
    }
}

fn main() {
    thread::spawn(socket_price);
    process()
}
