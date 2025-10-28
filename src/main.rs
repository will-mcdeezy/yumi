mod utils;

use axum::{ body::Body, extract::{ConnectInfo, State}, http::{ header::CONTENT_TYPE, Method, Request}, middleware::{self, Next}, response::{IntoResponse, Response,}, routing::{get, post}, Json, Router};

use serde_json::json;
use serde::{Serialize, Deserialize};
use solana_client::{rpc_client::{GetConfirmedSignaturesForAddress2Config, RpcClient}, rpc_config::RpcTransactionConfig, rpc_response::RpcConfirmedTransactionStatusWithSignature};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_transaction_status::UiTransactionEncoding;
use spl_associated_token_account::get_associated_token_address;
use utils::string_to_pub_key;
use std::{collections::HashMap, fmt::format, net::{IpAddr, SocketAddr}, str, sync::{Arc, Mutex}};
use chrono::{DateTime, Utc};
use tower_http::cors::{Any, CorsLayer};


#[derive(Serialize, Deserialize)]
struct RpcNetwork {
network: String
}

#[derive(Serialize, Deserialize)]
struct GetSolBalance {
network: String,
address: String,
}

#[derive(Serialize, Deserialize)]
struct IsTokenAcctActivated {
network: String,
address: String,
mint_address: String,
}

#[derive(Serialize, Deserialize)]
struct GetTokenBalance {
network: String,
address: String,
token_mint_address: String
}

#[derive(Serialize, Deserialize)]
struct GetTransaction {
    network: String,
    transaction: String // signature
}

#[derive(Serialize, Deserialize)]
struct GetAccountSignatures {
    address: String,
    network: String
}

// const REQUEST_LIMIT: usize = 60;
// #[derive(Clone, Default)]
// pub struct RateLimiter {
//     requests: Arc<Mutex<HashMap<IpAddr, Vec<DateTime<Utc>>>>>,
// }

// impl RateLimiter {
//     fn check_if_rate_limited(&self, ip_addr: IpAddr) -> Result<(), String> {
//         let throttle_time_limit = Utc::now() - std::time::Duration::from_secs(60);
        
//         let mut requests_hashmap = self.requests.lock().unwrap();
        
//         let requests_for_ip = requests_hashmap
//             .entry(ip_addr)
//             .or_insert(Vec::new());

//         requests_for_ip.retain(|x| x.to_utc() > throttle_time_limit);
//         requests_for_ip.push(Utc::now());

//         if requests_for_ip.len() > REQUEST_LIMIT {
//             return Err("IP is rate limited".to_string());
//         }

//         Ok(())
//     }
// }

// async fn rate_limit_middleware(
//     opt_connect_info: Option<ConnectInfo<SocketAddr>>,
//         State(rate_limiter): State<RateLimiter>,
//     request: Request<Body>,
//     next: Next,
// ) -> Response {
//     let ip = opt_connect_info
//     .map(|ConnectInfo(addr)| addr.ip())
//     .unwrap_or_else(|| "127.0.0.1".parse().unwrap());
    
//     match rate_limiter.check_if_rate_limited(ip) {
//         Ok(()) => next.run(request).await,
//         Err(_) => Json(json!({
//             "error": "Too many requests"
//         })).into_response()
//     }
// }


async fn get_recommended_fee() -> Response {
let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
let token_program = spl_token::id();  // Token program
let memo_program = spl_memo::id();    // Memo program
let usdc_mint = match string_to_pub_key("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v") { // USDC ADDRESS FOR MAINNET - REPLACE WITH DESIRED MINT ADDRESS TO SEE RECOMMEND FEES FOR GIVEN SPL TOKEN PROGRAM
    Ok(pubkey) => pubkey,
    Err(_) => return Json(json!({
        "error": "Failed to parse USDC mint address"
    })).into_response()
};

let recent_fees = match rpc.get_recent_prioritization_fees(&[
    token_program,
    memo_program,
    usdc_mint
]) {
    Ok(fees) => fees,
    Err(_) => return Json(json!({
        "error": "Failed to get fees"
    })).into_response()
};

// Sort fees to analyze distribution
let mut fees: Vec<u64> = recent_fees.iter()
    .map(|f| f.prioritization_fee)
    .filter(|&fee| fee > 0)
    .collect();
fees.sort_unstable();

if fees.is_empty() {
    return Json(json!({
        "error": "No valid fees found"
    })).into_response();
}

println!("fees: {:#?}", fees.iter().map(|f| f / 100_000).collect::<Vec<_>>());

let high_fee = fees[fees.len() * 50 / 100];   // 50th percentile

let high_cents = (high_fee as f64) / 100_000.0;

Json(json!({
    "data": {
                "fee": high_fee,
                "cents": high_cents,
        "status": "Successfully retrieved recommended priority fees"
    }
})).into_response()
}


async fn latesthash(Json(payload): Json<RpcNetwork>) ->  Response {

    let rpc_url = format(format_args!("https://api.{}.solana.com", payload.network));
    let rpc = RpcClient::new(rpc_url);
    let blockhash = rpc.get_latest_blockhash().unwrap();
    let str_blockhash = blockhash.to_string();
 
    Json(json!({
        "data": str_blockhash,
        "status": "Successful blockhash"
    })).into_response()
}

 async fn get_sol_balance_processed(Json(payload): Json<GetSolBalance>) -> Response {

    let rpc_url = format(format_args!("https://api.{}.solana.com", payload.network));
    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::processed());
    let user_pubkey = match utils::string_to_pub_key(&payload.address) {
    Ok(pubkey) => pubkey,
    Err(_) => return Json(json!({
        "error": "Invalid wallet address"
    })).into_response()
};
   
    let balance = match rpc.get_balance(&user_pubkey) {
        Ok(balance) => balance,
        Err(_) => return Json(json!({
            "error": "Could not retrieve balance"
        })).into_response()
    };
 
    Json(json!({
        "data": balance,
        "status": "Successful solana balance request"
    })).into_response()
}


async fn get_sol_balance_confirmed(Json(payload): Json<GetSolBalance>) -> Response {

    let rpc_url = format(format_args!("https://api.{}.solana.com", payload.network));
    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
    let user_pubkey = match utils::string_to_pub_key(&payload.address) {
    Ok(pubkey) => pubkey,
    Err(_) => return Json(json!({
        "error": "Invalid wallet address"
    })).into_response()
};
   
    let balance = match rpc.get_balance(&user_pubkey) {
        Ok(balance) => balance,
        Err(_) => return Json(json!({
            "error": "Could not retrieve balance"
        })).into_response()
    };
 
    Json(json!({
        "data": balance,
        "status": "Successful solana balance request"
    })).into_response()
}


async fn get_sol_balance_finalized(Json(payload): Json<GetSolBalance>) -> Response {

    let rpc_url = format(format_args!("https://api.{}.solana.com", payload.network));
    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::finalized());
    let user_pubkey = match utils::string_to_pub_key(&payload.address) {
    Ok(pubkey) => pubkey,
    Err(_) => return Json(json!({
        "error": "Invalid wallet address"
    })).into_response()
};
   
    let balance = match rpc.get_balance(&user_pubkey) {
        Ok(balance) => balance,
        Err(_) => return Json(json!({
            "error": "Could not retrieve balance"
        })).into_response()
    };
 
    Json(json!({
        "data": balance,
        "status": "Successful solana balance request"
    })).into_response()
}

async fn get_token_balance_processed(Json(payload): Json<GetTokenBalance>) -> Response {

    let rpc_url = format(format_args!("https://api.{}.solana.com", payload.network));
    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::processed());
    let user_pubkey = match utils::string_to_pub_key(&payload.address) {
    Ok(pubkey) => pubkey,
    Err(_) => return Json(json!({
        "error": "Invalid wallet address"
    })).into_response()
};

let token_mint_pubkey = match utils::string_to_pub_key(&payload.token_mint_address) {
    Ok(pubkey) => pubkey,
    Err(_) => return Json(json!({
        "error": "Invalid token address"
    })).into_response()
};

let associated_token_pubkey: solana_sdk::pubkey::Pubkey = get_associated_token_address(&user_pubkey, &token_mint_pubkey);

   
    let balance = match rpc.get_token_account_balance(&associated_token_pubkey) {
        Ok(balance) => balance,
        Err(_) => return Json(json!({
            "error": "Could not retrieve balance for token"
        })).into_response()
    };
 
    Json(json!({
        "data": {
            "baseAmount": balance.amount,
            "displayAmount": balance.ui_amount_string,
            "decimals": balance.decimals
        },
        "status": "Successful balance request"
    })).into_response()
}

async fn get_token_balance_confirmed(Json(payload): Json<GetTokenBalance>) -> Response {

    let rpc_url = format(format_args!("https://api.{}.solana.com", payload.network));
    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
    let user_pubkey = match utils::string_to_pub_key(&payload.address) {
    Ok(pubkey) => pubkey,
    Err(_) => return Json(json!({
        "error": "Invalid wallet address"
    })).into_response()
};

let token_mint_pubkey = match utils::string_to_pub_key(&payload.token_mint_address) {
    Ok(pubkey) => pubkey,
    Err(_) => return Json(json!({
        "error": "Invalid token address"
    })).into_response()
};

let associated_token_pubkey: solana_sdk::pubkey::Pubkey = get_associated_token_address(&user_pubkey, &token_mint_pubkey);
   
    let balance = match rpc.get_token_account_balance(&associated_token_pubkey) {
        Ok(balance) => balance,
        Err(_) => return Json(json!({
            "error": "Could not retrieve balance for token"
        })).into_response()
    };
 
    Json(json!({
        "data": {
            "baseAmount": balance.amount,
            "displayAmount": balance.ui_amount_string,
            "decimals": balance.decimals
        },
        "status": "Successful balance request"
    })).into_response()
}

async fn get_token_balance_finalized(Json(payload): Json<GetTokenBalance>) -> Response {

    let rpc_url = format(format_args!("https://api.{}.solana.com", payload.network));
    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::finalized());
    let user_pubkey = match utils::string_to_pub_key(&payload.address) {
    Ok(pubkey) => pubkey,
    Err(_) => return Json(json!({
        "error": "Invalid wallet address"
    })).into_response()
};

    let token_mint_pubkey = match utils::string_to_pub_key(&payload.token_mint_address) {
    Ok(pubkey) => pubkey,
    Err(_) => return Json(json!({
        "error": "Invalid token address"
    })).into_response()
};

    let associated_token_pubkey: solana_sdk::pubkey::Pubkey = get_associated_token_address(&user_pubkey, &token_mint_pubkey);

    let balance = match rpc.get_token_account_balance(&associated_token_pubkey) {
        Ok(balance) => balance,
        Err(_) => return Json(json!({
            "error": "Could not retrieve balance for token"
        })).into_response()
    };
 
    Json(json!({
        "data": {
            "baseAmount": balance.amount,
            "displayAmount": balance.ui_amount_string,
            "decimals": balance.decimals
        },
        "status": "Successful balance request"
    })).into_response()
}

async fn get_transaction_confirmed(Json(payload): Json<GetTransaction>) -> Response {

    let rpc_url = format(format_args!("https://api.{}.solana.com", payload.network));
    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let signature = match utils::string_to_signature(&payload.transaction) {
        Ok(sig) => sig,
            Err(_) => return Json(json!({
                "error": "Invalid Signature String"
            })).into_response()
        };

    let config = RpcTransactionConfig {
        encoding: Some(UiTransactionEncoding::Json),
        commitment: Some(CommitmentConfig::confirmed()),
        max_supported_transaction_version: Some(0),
    };

    let transaction = match rpc.get_transaction_with_config(
        &signature,
        config,
    )  { Ok(transaction) => transaction,
    Err(_) => return Json(json!({
        "error": "Could not retrieve transaction for signature"
    })).into_response()
};

    Json(json!({
        "data": transaction,
        "status": "Successful transaction confirmed"
    })).into_response()
}

async fn get_account_signatures_for_arrow_api_confirmed(Json(payload): Json<GetAccountSignatures>) -> Response {

    let rpc_url = format(format_args!("https://api.{}.solana.com", payload.network));
    let rpc = RpcClient::new(rpc_url);

    let user_pubkey = match utils::string_to_pub_key(&payload.address) {
        Ok(pubkey) => pubkey,
        Err(_) => return Json(json!({
            "error": "Invalid wallet address"
        })).into_response()
    };

    let config = GetConfirmedSignaturesForAddress2Config {
        before: None,
        until: None,
        limit: None,
        commitment: Some(CommitmentConfig::confirmed()),
    };

    let signatures = match rpc.get_signatures_for_address_with_config(
        &user_pubkey,config
    )  
        { Ok(signatures) => signatures,
    Err(_) => return Json(json!({
        "error": "Could not retrieve signatures for account"
    })).into_response()
};

let memos: Vec<RpcConfirmedTransactionStatusWithSignature> = signatures
    .into_iter()
    .filter(|sig_info| {
        sig_info.memo.as_ref().map_or(false, |memo| memo.contains("arrow-api"))
    })
    .collect();

    Json(json!({
        "data": memos,
        "status": "Successful account signatures for arrow-api received"
    })).into_response()
}

async fn get_latest_signature_confirmed(Json(payload): Json<GetAccountSignatures>) -> Response {

    let rpc_url = format(format_args!("https://api.{}.solana.com", payload.network));
    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let user_pubkey = match utils::string_to_pub_key(&payload.address) {
        Ok(pubkey) => pubkey,
        Err(_) => return Json(json!({
            "error": "Invalid wallet address"
        })).into_response()
    };

    let config = GetConfirmedSignaturesForAddress2Config {
        before: None,
        until: None,
        limit: Some(1),
        commitment: Some(CommitmentConfig::confirmed())

    };

    let signatures = match rpc.get_signatures_for_address_with_config(
        &user_pubkey, config)  
        { Ok(signatures) => signatures,
    Err(_) => return Json(json!({
        "error": "Could not retrieve signatures for account"
    })).into_response()
};

let memos: Vec<RpcConfirmedTransactionStatusWithSignature> = signatures
    .into_iter()
    .filter(|sig_info| {
        sig_info.memo.as_ref().map_or(false, |memo| memo.contains("arrow-api"))
    })
    .collect();

    Json(json!({
        "data": memos,
        "status": "Successful latest signature for arrow-api received"
    })).into_response()
}


async fn get_is_spl_activated(Json(payload): Json<IsTokenAcctActivated>) -> Response {

    let rpc_url = format(format_args!("https://api.{}.solana.com", payload.network));
    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());
    let user_pubkey = match utils::string_to_pub_key(&payload.address) {
    Ok(pubkey) => pubkey,
    Err(_) => return Json(json!({
        "error": "Invalid wallet address"
    })).into_response()
};

let token_pubkey = match utils::string_to_pub_key(&payload.mint_address)  {
    Ok(pubkey) => pubkey,
    Err(_) => return Json(json!({
        "error": "Invalid pubkey address for fee-des"
    })).into_response(),
};

let associated_account_pubkey = get_associated_token_address(&user_pubkey, &token_pubkey);

   
    let balance = match rpc.get_balance(&associated_account_pubkey) {
        Ok(balance) => balance,
        Err(_) => return Json(json!({
            "error": "Could not retrieve balance"
        })).into_response()
    };

    Json(json!({
        "data": balance,
        "isActivated": balance >= 2039280, // Rent needed to activate associated token account
        "status": "Token account activation query successful"
    })).into_response()
}


#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    // let rate_limiter = RateLimiter::default();
    

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE])
        .allow_origin(Any);
    let app = Router::new()
        .route("/", get(||  async { "Yatori" }))
        .route("/blockhash", post(latesthash))
        .route("/sol-balance-processed", post(get_sol_balance_processed))
        .route("/sol-balance-confirmed", post(get_sol_balance_confirmed))
        .route("/sol-balance-finalized", post(get_sol_balance_finalized))
        .route("/token-balance-processed", post(get_token_balance_processed))
        .route("/token-balance-confirmed", post(get_token_balance_confirmed))
        .route("/token-balance-finalized", post(get_token_balance_finalized))
        .route("/get-transaction-confirmed", post(get_transaction_confirmed))
        .route("/get-arrow-acc-sigs", post(get_account_signatures_for_arrow_api_confirmed))
        .route("/get-latest-sig", post(get_latest_signature_confirmed))
        .route("/is-usdc-acct-activated", post(get_is_spl_activated))
        .route("/get-rec-fee", get(get_recommended_fee))
        .layer(cors);
        // .with_state(rate_limiter.clone());
        // .layer(middleware::from_fn_with_state(rate_limiter, rate_limit_middleware));

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
   
    axum::serve(listener, app).await.unwrap();
}
