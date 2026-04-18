// ============================================================
// بوت تحليل Solana المتقدم - ملف واحد كامل
// ضع API Keys الخاصة بك في قسم CONFIG أدناه
// ============================================================

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use chrono::{DateTime, Utc};
use std::fs;


// ============================================================
// CONFIG - ضع مفاتيحك هنا
// ============================================================
const HELIUS_API_KEY: &str = "0a47e81d-8e80-4aed-8505-5e5176f3540f";
const TELEGRAM_BOT_TOKEN: &str = "7839767838:AAGbRydF792cf8fue_HkUNnNUzVWWdrB-3I";
const TELEGRAM_CHAT_ID: &str = "2010797927";

const SCAN_INTERVAL_SECS: u64 = 30;
const PROFIT_CHECK_INTERVAL_SECS: u64 = 300;
const SAVE_INTERVAL_MINS: i64 = 10;
const MAX_TOKEN_AGE_HOURS: i64 = 6;
const MIN_SCORE_NEW_TOKEN: f64 = 80.0;
const MIN_SCORE_OLDER_TOKEN: f64 = 85.0;
const MAX_DAILY_ALERTS: u32 = 15;

// ============================================================
// STRUCTURES - DexScreener
// ============================================================

#[derive(Debug, Deserialize, Clone)]
struct DexToken {
    #[serde(rename = "chainId")]
    chain_id: String,
    #[serde(rename = "tokenAddress")]
    token_address: String,
    name: Option<String>,
    symbol: Option<String>,
    description: Option<String>,
    #[serde(rename = "imageUrl")]
    image_url: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone)]
struct PairData {
    #[serde(rename = "chainId")]
    chain_id: String,
    #[serde(rename = "dexId")]
    dex_id: String,
    url: String,
    #[serde(rename = "pairAddress")]
    pair_address: String,
    #[serde(rename = "baseToken")]
    base_token: TokenBasicInfo,
    #[serde(rename = "priceUsd")]
    price_usd: Option<String>,
    #[serde(rename = "pairCreatedAt")]
    pair_created_at: Option<i64>,
    liquidity: Option<LiquidityInfo>,
    volume: Option<VolumeInfo>,
    #[serde(rename = "priceChange")]
    price_change: Option<PriceChangeInfo>,
    fdv: Option<f64>,
    #[serde(rename = "marketCap")]
    market_cap: Option<f64>,
    txns: Option<TxnsInfo>,
}

#[derive(Debug, Deserialize, Clone)]
struct TokenBasicInfo {
    address: String,
    name: String,
    symbol: String,
}

#[derive(Debug, Deserialize, Clone)]
struct LiquidityInfo {
    usd: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
struct VolumeInfo {
    #[serde(rename = "h24")]
    h24: Option<f64>,
    #[serde(rename = "h1")]
    h1: Option<f64>,
    #[serde(rename = "m5")]
    m5: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
struct PriceChangeInfo {
    #[serde(rename = "m5")]
    m5: Option<f64>,
    #[serde(rename = "h1")]
    h1: Option<f64>,
    #[serde(rename = "h6")]
    h6: Option<f64>,
    #[serde(rename = "h24")]
    h24: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
struct TxnsInfo {
    #[serde(rename = "h1")]
    h1: Option<TxnData>,
    #[serde(rename = "m5")]
    m5: Option<TxnData>,
}

#[derive(Debug, Deserialize, Clone)]
struct TxnData {
    buys: Option<u32>,
    sells: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct PairResponse {
    pairs: Option<Vec<PairData>>,
}

// ============================================================
// STRUCTURES - Helius API
// ============================================================

#[derive(Debug, Deserialize)]
struct HeliusTokenInfo {
    #[serde(rename = "mintAuthority")]
    mint_authority: Option<String>,
    #[serde(rename = "freezeAuthority")]
    freeze_authority: Option<String>,
    decimals: Option<u8>,
    supply: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HeliusTokenHolder {
    address: String,
    amount: String,
}

#[derive(Debug, Deserialize)]
struct HeliusTransaction {
    signature: String,
    timestamp: i64,
    #[serde(rename = "feePayer")]
    fee_payer: Option<String>,
    #[serde(rename = "tokenTransfers")]
    token_transfers: Option<Vec<TokenTransfer>>,
}

#[derive(Debug, Deserialize, Clone)]
struct TokenTransfer {
    #[serde(rename = "fromUserAccount")]
    from_user_account: Option<String>,
    #[serde(rename = "toUserAccount")]
    to_user_account: Option<String>,
    #[serde(rename = "tokenAmount")]
    token_amount: f64,
    mint: Option<String>,
}

// ============================================================
// STRUCTURES - Analysis Results
// ============================================================

#[derive(Debug, Clone)]
struct HolderAnalysis {
    total_holders: u32,
    sniper_count: u32,
    sniper_percentage: f64,
    top10_concentration: f64,
    gini_coefficient: f64,
    bundled_wallets_detected: bool,
    developer_sold_percentage: f64,
    score: f64, // 0-25
    flags: Vec<String>,
    signals: Vec<String>,
}

#[derive(Debug, Clone)]
struct LiquidityAnalysis {
    total_usd: f64,
    lp_burned: bool,
    lp_locked: bool,
    lock_duration_months: Option<f64>,
    independent_providers: u32,
    price_impact_5k: f64,
    price_impact_10k: f64,
    price_impact_25k: f64,
    score: f64, // 0-20
    flags: Vec<String>,
    signals: Vec<String>,
}

#[derive(Debug, Clone)]
struct WhaleAnalysis {
    smart_wallets_entered: u32,
    accumulation_pattern: bool,
    distribution_signs: bool,
    cold_storage_transfers: u32,
    largest_single_buy_usd: f64,
    score: f64, // 0-20
    signals: Vec<String>,
}

#[derive(Debug, Clone)]
struct TechnicalAnalysis {
    momentum_5m: f64,
    momentum_1h: f64,
    momentum_6h: f64,
    momentum_24h: f64,
    volume_24h: f64,
    buy_pressure_ratio: f64,
    pattern_detected: Option<String>,
    support_level: Option<f64>,
    resistance_level: Option<f64>,
    score: f64, // 0-10
    signals: Vec<String>,
}

#[derive(Debug, Clone)]
struct ContractSecurity {
    mint_authority_revoked: bool,
    freeze_authority_revoked: bool,
    transfer_fee_percent: f64,
    honeypot_risk: bool,
    score: f64, // 0-10
    flags: Vec<String>,
    signals: Vec<String>,
}

#[derive(Debug, Clone)]
struct SocialAnalysis {
    has_twitter: bool,
    has_telegram: bool,
    social_hype_score: f64,
    score: f64, // 0-15
    signals: Vec<String>,
}

#[derive(Debug, Clone)]
struct TokenLifecycle {
    age_minutes: i64,
    phase: LifecyclePhase,
    entry_timing_score: f64,
    risk_reward_ratio: f64,
}

#[derive(Debug, Clone, PartialEq)]
enum LifecyclePhase {
    Launch,       // 0-30 min - danger
    FirstDip,     // 30min - 3hr - opportunity
    Accumulation, // 3-12 hr - best
    Breakout,     // 12-48 hr - late
    Mature,       // 48hr+ - too late
}

impl LifecyclePhase {
    fn to_arabic(&self) -> &str {
        match self {
            LifecyclePhase::Launch => "⚡ مرحلة الإطلاق (خطيرة)",
            LifecyclePhase::FirstDip => "📉 التصحيح الأول (فرصة)",
            LifecyclePhase::Accumulation => "🟢 مرحلة التجميع (مثالية)",
            LifecyclePhase::Breakout => "🔥 مرحلة الاختراق (متأخر)",
            LifecyclePhase::Mature => "⬜ ناضج (متأخر جداً)",
        }
    }
}

#[derive(Debug, Clone)]
struct FullTokenAnalysis {
    token_address: String,
    symbol: String,
    name: String,
    image_url: Option<String>,
    price_usd: Option<f64>,
    market_cap: Option<f64>,
    dex_urls: Vec<String>,
    total_score: f64,
    confidence_level: ConfidenceLevel,
    potential_multiplier: String,
    holder_analysis: HolderAnalysis,
    liquidity_analysis: LiquidityAnalysis,
    whale_analysis: WhaleAnalysis,
    technical_analysis: TechnicalAnalysis,
    contract_security: ContractSecurity,
    social_analysis: SocialAnalysis,
    lifecycle: TokenLifecycle,
    alert_level: AlertLevel,
    all_red_flags: Vec<String>,
    top_signals: Vec<String>,
}

#[derive(Debug, Clone)]
enum ConfidenceLevel {
    VeryHigh, // 90+
    High,     // 85-89
    Medium,   // 80-84
    Low,      // 75-79
}

#[derive(Debug, Clone)]
enum AlertLevel {
    Legendary,  // 90+
    Golden,     // 85-89
    Excellent,  // 80-84
    Normal,     // 75-79
}

impl AlertLevel {
    fn to_header(&self) -> &str {
        match self {
            AlertLevel::Legendary => "👑 **فرصة أسطورية** 👑",
            AlertLevel::Golden => "💎 **فرصة ذهبية** 💎",
            AlertLevel::Excellent => "🔥 **فرصة ممتازة** 🔥",
            AlertLevel::Normal => "⭐ **فرصة عادية** ⭐",
        }
    }
}

// ============================================================
// STRUCTURES - Tracking & Persistence
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrackedToken {
    token_address: String,
    symbol: String,
    name: String,
    image_url: Option<String>,
    initial_price: f64,
    highest_price: f64,
    discovery_time: DateTime<Utc>,
    milestones_reached: Vec<u32>, // 50, 100, 200, 500, 1000, 2000
}

#[derive(Debug, Serialize, Deserialize)]
struct BotPersistentData {
    seen_tokens: HashMap<String, String>, // address -> ISO timestamp
    tracked_tokens: HashMap<String, TrackedToken>,
    smart_wallets: Vec<String>,
    daily_alert_count: u32,
    last_reset_date: String,
    performance_stats: PerformanceStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PerformanceStats {
    total_alerts_sent: u32,
    tokens_reached_2x: u32,
    tokens_reached_5x: u32,
    tokens_reached_10x: u32,
    best_token_symbol: String,
    best_token_gain_percent: f64,
}

// ============================================================
// STRUCTURES - Telegram
// ============================================================

#[derive(Debug, Serialize)]
struct TelegramMsg {
    chat_id: String,
    text: String,
    parse_mode: String,
    disable_web_page_preview: bool,
}

#[derive(Debug, Serialize)]
struct TelegramPhoto {
    chat_id: String,
    photo: String,
    caption: String,
    parse_mode: String,
}

#[derive(Debug, Serialize)]
struct TelegramInlineKeyboard {
    chat_id: String,
    text: String,
    parse_mode: String,
    reply_markup: InlineKeyboardMarkup,
}

#[derive(Debug, Serialize)]
struct InlineKeyboardMarkup {
    inline_keyboard: Vec<Vec<InlineButton>>,
}

#[derive(Debug, Serialize)]
struct InlineButton {
    text: String,
    url: Option<String>,
    callback_data: Option<String>,
}

// ============================================================
// RATE LIMITER
// ============================================================

struct RateLimiter {
    calls: Vec<Instant>,
    max_calls: usize,
    window_secs: u64,
}

impl RateLimiter {
    fn new(max_calls: usize, window_secs: u64) -> Self {
        Self { calls: Vec::new(), max_calls, window_secs }
    }

    async fn wait_if_needed(&mut self) {
        let now = Instant::now();
        let window = Duration::from_secs(self.window_secs);
        self.calls.retain(|t| now.duration_since(*t) < window);
        if self.calls.len() >= self.max_calls {
            let oldest = self.calls[0];
            let wait = window.checked_sub(now.duration_since(oldest))
                .unwrap_or(Duration::from_millis(100));
            sleep(wait).await;
            self.calls.retain(|t| Instant::now().duration_since(*t) < window);
        }
        self.calls.push(Instant::now());
    }
}

// ============================================================
// MAIN BOT STRUCT
// ============================================================

struct SolanaBot {
    client: Client,
    data: BotPersistentData,
    dex_limiter: RateLimiter,
    helius_limiter: RateLimiter,
    tg_limiter: RateLimiter,
    last_save: DateTime<Utc>,
    is_paused: bool,
}

impl SolanaBot {
    fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .tcp_keepalive(Duration::from_secs(60))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            data: BotPersistentData {
                seen_tokens: HashMap::new(),
                tracked_tokens: HashMap::new(),
                smart_wallets: Self::initial_smart_wallets(),
                daily_alert_count: 0,
                last_reset_date: Utc::now().format("%Y-%m-%d").to_string(),
                performance_stats: PerformanceStats::default(),
            },
            dex_limiter: RateLimiter::new(55, 60),
            helius_limiter: RateLimiter::new(40, 60),
            tg_limiter: RateLimiter::new(20, 60),
            last_save: Utc::now(),
            is_paused: false,
        }
    }

    fn initial_smart_wallets() -> Vec<String> {
        // أضف هنا محافظ الحيتان الذكية المعروفة تاريخياً
        vec![
            // "wallet_address_1".to_string(),
            // "wallet_address_2".to_string(),
        ]
    }

    // ============================================================
    // PERSISTENCE
    // ============================================================

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self.data)?;
        fs::write("bot_data.json", json)?;
        println!("💾 تم حفظ البيانات - {} عملة مرصودة, {} متتبعة",
            self.data.seen_tokens.len(),
            self.data.tracked_tokens.len());
        Ok(())
    }

    fn load(&mut self) {
        match fs::read_to_string("bot_data.json") {
            Ok(content) => {
                match serde_json::from_str::<BotPersistentData>(&content) {
                    Ok(data) => {
                        self.data = data;
                        // تنظيف البيانات القديمة (أكثر من 30 يوم)
                        let cutoff = Utc::now() - chrono::Duration::days(30);
                        self.data.seen_tokens.retain(|_, ts| {
                            ts.parse::<DateTime<Utc>>()
                                .map(|t| t > cutoff)
                                .unwrap_or(false)
                        });
                        println!("📂 تم تحميل البيانات: {} عملة مرصودة, {} متتبعة",
                            self.data.seen_tokens.len(),
                            self.data.tracked_tokens.len());
                    }
                    Err(e) => println!("⚠️ خطأ في تحليل ملف البيانات: {}", e),
                }
            }
            Err(_) => println!("ℹ️ بدء جديد - لا توجد بيانات محفوظة"),
        }
    }

    fn reset_daily_count_if_needed(&mut self) {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        if self.data.last_reset_date != today {
            self.data.daily_alert_count = 0;
            self.data.last_reset_date = today;
        }
    }

    // ============================================================
    // TELEGRAM
    // ============================================================

    async fn send_message(&mut self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.tg_limiter.wait_if_needed().await;
        let url = format!("https://api.telegram.org/bot{}/sendMessage", TELEGRAM_BOT_TOKEN);
        let payload = TelegramMsg {
            chat_id: TELEGRAM_CHAT_ID.to_string(),
            text: text.to_string(),
            parse_mode: "Markdown".to_string(),
            disable_web_page_preview: true,
        };
        let resp = self.client.post(&url).json(&payload).send().await?;
        if !resp.status().is_success() {
            let err = resp.text().await?;
            return Err(format!("Telegram error: {}", err).into());
        }
        Ok(())
    }

    async fn send_photo(&mut self, photo_url: &str, caption: &str) {
        self.tg_limiter.wait_if_needed().await;
        let url = format!("https://api.telegram.org/bot{}/sendPhoto", TELEGRAM_BOT_TOKEN);
        let payload = TelegramPhoto {
            chat_id: TELEGRAM_CHAT_ID.to_string(),
            photo: photo_url.to_string(),
            caption: caption.to_string(),
            parse_mode: "Markdown".to_string(),
        };
        if let Err(e) = self.client.post(&url).json(&payload).send().await {
            println!("❌ خطأ في إرسال الصورة: {}", e);
        }
    }

    async fn send_alert_with_buttons(&mut self, text: &str, dex_url: &str) {
        self.tg_limiter.wait_if_needed().await;
        let url = format!("https://api.telegram.org/bot{}/sendMessage", TELEGRAM_BOT_TOKEN);
        let payload = TelegramInlineKeyboard {
            chat_id: TELEGRAM_CHAT_ID.to_string(),
            text: text.to_string(),
            parse_mode: "Markdown".to_string(),
            reply_markup: InlineKeyboardMarkup {
                inline_keyboard: vec![
                    vec![
                        InlineButton {
                            text: "📊 DexScreener".to_string(),
                            url: Some(dex_url.to_string()),
                            callback_data: None,
                        },
                    ],
                    vec![
                        InlineButton {
                            text: "⏸ إيقاف مؤقت".to_string(),
                            url: None,
                            callback_data: Some("/pause".to_string()),
                        },
                        InlineButton {
                            text: "📈 الإحصاءات".to_string(),
                            url: None,
                            callback_data: Some("/stats".to_string()),
                        },
                    ],
                ],
            },
        };
        if let Err(e) = self.client.post(&url).json(&payload).send().await {
            println!("❌ خطأ في إرسال الرسالة مع الأزرار: {}", e);
        }
    }

    // ============================================================
    // DEXSCREENER API
    // ============================================================

    async fn get_new_solana_tokens(&mut self) -> Result<Vec<DexToken>, Box<dyn std::error::Error>> {
        self.dex_limiter.wait_if_needed().await;
        let url = "https://api.dexscreener.com/token-profiles/latest/v1";
        let resp = self.client.get(url).send().await?;
        if !resp.status().is_success() {
            return Ok(vec![]);
        }
        let tokens: Vec<DexToken> = resp.json().await.unwrap_or_default();
        let now = Utc::now().timestamp();
        Ok(tokens.into_iter().filter(|t| {
            if t.chain_id != "solana" { return false; }
            if let Some(created) = &t.created_at {
                let ts = match created {
                    serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
                    serde_json::Value::String(s) => s.parse().unwrap_or(0),
                    _ => 0,
                };
                if ts > 0 {
                    let age_hours = (now - ts) / 3600;
                    return age_hours <= MAX_TOKEN_AGE_HOURS;
                }
            }
            true // إذا لم يكن هناك وقت إنشاء، نفترض أنها جديدة
        }).collect())
    }

    async fn get_pairs(&mut self, address: &str) -> Vec<PairData> {
        self.dex_limiter.wait_if_needed().await;
        let url = format!("https://api.dexscreener.com/latest/dex/tokens/{}", address);
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.json::<PairResponse>().await {
                    Ok(r) => r.pairs.unwrap_or_default()
                        .into_iter()
                        .filter(|p| p.chain_id == "solana")
                        .collect(),
                    Err(_) => vec![],
                }
            }
            _ => vec![],
        }
    }

    async fn get_boosted_tokens(&mut self) -> Vec<String> {
        self.dex_limiter.wait_if_needed().await;
        let url = "https://api.dexscreener.com/token-boosts/latest/v1";
        #[derive(Deserialize)]
        struct BoostToken { #[serde(rename = "tokenAddress")] address: String, #[serde(rename = "chainId")] chain: String }
        match self.client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                resp.json::<Vec<BoostToken>>().await.unwrap_or_default()
                    .into_iter()
                    .filter(|t| t.chain == "solana")
                    .map(|t| t.address)
                    .collect()
            }
            _ => vec![],
        }
    }

    // ============================================================
    // HELIUS API
    // ============================================================

    async fn get_token_metadata(&mut self, address: &str) -> Option<HeliusTokenInfo> {
        self.helius_limiter.wait_if_needed().await;
        let url = format!(
            "https://api.helius.xyz/v0/token-metadata?api-key={}", HELIUS_API_KEY
        );
        let body = serde_json::json!({ "mintAccounts": [address] });
        match self.client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                let arr: Vec<serde_json::Value> = resp.json().await.ok()?;
                let item = arr.into_iter().next()?;
                let _on_chain = item.get("onChainMetadata")?
                    .get("metadata")?
                    .get("tokenStandard");
                // نستخرج mint/freeze authority من account info
                let mint_auth = item.get("account")
                    .and_then(|a| a.get("data"))
                    .and_then(|d| d.get("parsed"))
                    .and_then(|p| p.get("info"))
                    .and_then(|i| i.get("mintAuthority"))
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string());
                let freeze_auth = item.get("account")
                    .and_then(|a| a.get("data"))
                    .and_then(|d| d.get("parsed"))
                    .and_then(|p| p.get("info"))
                    .and_then(|i| i.get("freezeAuthority"))
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string());
                let supply = item.get("account")
                    .and_then(|a| a.get("data"))
                    .and_then(|d| d.get("parsed"))
                    .and_then(|p| p.get("info"))
                    .and_then(|i| i.get("supply"))
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());
                Some(HeliusTokenInfo {
                    mint_authority: mint_auth,
                    freeze_authority: freeze_auth,
                    decimals: None,
                    supply,
                })
            }
            _ => None,
        }
    }

    async fn get_token_holders(&mut self, address: &str) -> Vec<HeliusTokenHolder> {
        self.helius_limiter.wait_if_needed().await;
        // Helius: نستخدم getTokenLargestAccounts عبر JSON-RPC
        let url = format!("https://mainnet.helius-rpc.com/?api-key={}", HELIUS_API_KEY);
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getTokenLargestAccounts",
            "params": [address, {"commitment": "finalized"}]
        });
        match self.client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => {
                let val: serde_json::Value = resp.json().await.unwrap_or_default();
                val.get("result")
                    .and_then(|r| r.get("value"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter().filter_map(|item| {
                            Some(HeliusTokenHolder {
                                address: item.get("address")?.as_str()?.to_string(),
                                amount: item.get("uiAmountString")
                                    .or_else(|| item.get("amount"))
                                    .and_then(|a| a.as_str())
                                    .unwrap_or("0")
                                    .to_string(),
                            })
                        }).collect()
                    })
                    .unwrap_or_default()
            }
            _ => vec![],
        }
    }

    async fn get_recent_transactions(&mut self, address: &str) -> Vec<HeliusTransaction> {
        self.helius_limiter.wait_if_needed().await;
        let url = format!(
            "https://api.helius.xyz/v0/addresses/{}/transactions?api-key={}&limit=50&type=SWAP",
            address, HELIUS_API_KEY
        );
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                resp.json::<Vec<HeliusTransaction>>().await.unwrap_or_default()
            }
            _ => vec![],
        }
    }

    // ============================================================
    // ANALYSIS MODULES
    // ============================================================

    // --- 1. تحليل الحاملين ---
    async fn analyze_holders(&mut self, address: &str, pairs: &[PairData]) -> HolderAnalysis {
        let holders = self.get_token_holders(address).await;
        let txns = self.get_recent_transactions(address).await;
        let now = Utc::now().timestamp();

        let total_holders = holders.len() as u32;
        let mut sniper_count = 0u32;
        let mut dev_sold_pct = 0.0f64;
        let mut flags = vec![];
        let mut signals = vec![];

        // حساب إجمالي العرض من أكبر الحاملين
        let amounts: Vec<f64> = holders.iter()
            .filter_map(|h| h.amount.parse::<f64>().ok())
            .collect();
        let total_supply: f64 = amounts.iter().sum();

        // أكبر 10 محافظ
        let mut sorted_amounts = amounts.clone();
        sorted_amounts.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let top10_sum: f64 = sorted_amounts.iter().take(10).sum();
        let top10_concentration = if total_supply > 0.0 {
            top10_sum / total_supply * 100.0
        } else { 0.0 };

        // كشف السنابرز (اشتروا في أول 30 ثانية من إنشاء الزوج)
        let pair_created = pairs.first()
            .and_then(|p| p.pair_created_at)
            .unwrap_or(now - 3600);
        for txn in &txns {
            if txn.timestamp > pair_created && txn.timestamp < pair_created + 30 {
                sniper_count += 1;
            }
        }
        let sniper_pct = if total_holders > 0 {
            sniper_count as f64 / total_holders as f64 * 100.0
        } else { 0.0 };

        // كشف المحافظ المجمعة (bundle detection)
        let mut timestamp_groups: HashMap<i64, u32> = HashMap::new();
        for txn in &txns {
            *timestamp_groups.entry(txn.timestamp / 2).or_insert(0) += 1;
        }
        let bundled = timestamp_groups.values().any(|&count| count >= 3);

        // حساب Gini Coefficient
        let gini = calculate_gini(&amounts);

        // كشف بيع المطور (أول حامل في txns باع)
        if !txns.is_empty() {
            let first_buyer = txns.last().and_then(|t| t.fee_payer.clone());
            if let Some(dev_wallet) = first_buyer {
                let dev_sales: f64 = txns.iter()
                    .filter(|t| t.fee_payer.as_deref() == Some(&dev_wallet))
                    .filter_map(|t| t.token_transfers.as_ref())
                    .flatten()
                    .filter(|tf| tf.from_user_account.as_deref() == Some(&dev_wallet))
                    .map(|tf| tf.token_amount)
                    .sum();
                dev_sold_pct = if total_supply > 0.0 { dev_sales / total_supply * 100.0 } else { 0.0 };
            }
        }

        // العلامات الحمراء
        if sniper_pct > 20.0 {
            flags.push(format!("🔴 سنابرز عالية: {:.1}%", sniper_pct));
        }
        if bundled {
            flags.push("🔴 محافظ مجمعة (Bundled) مكتشفة".to_string());
        }
        if dev_sold_pct > 10.0 {
            flags.push(format!("🔴 المطور باع {:.1}% من حصته", dev_sold_pct));
        }
        if top10_concentration > 30.0 {
            flags.push(format!("🔴 تركيز عالٍ: أكبر 10 محافظ تمتلك {:.1}%", top10_concentration));
        }
        if gini > 0.6 {
            flags.push(format!("🔴 توزيع غير عادل (Gini: {:.2})", gini));
        }

        // الإشارات الإيجابية
        if sniper_pct < 15.0 && !bundled && dev_sold_pct < 10.0 {
            signals.push(format!("✅ توزيع حاملين صحي - سنابرز: {:.1}%", sniper_pct));
        }
        if top10_concentration < 25.0 {
            signals.push(format!("✅ توزيع ممتاز - أكبر 10 محافظ: {:.1}%", top10_concentration));
        }
        if total_holders >= 500 {
            signals.push(format!("👥 قاعدة حاملين قوية: {} حامل", total_holders));
        } else if total_holders >= 100 {
            signals.push(format!("👥 {} حامل", total_holders));
        }

        // الحساب النهائي (max 25 نقطة)
        let mut score: f64 = 25.0;
        if sniper_pct > 20.0 { score -= 10.0; }
        else if sniper_pct > 15.0 { score -= 5.0; }
        if bundled { score -= 8.0; }
        if dev_sold_pct > 10.0 { score -= 7.0; }
        if top10_concentration > 30.0 { score -= 5.0; }
        if gini > 0.6 { score -= 3.0; }
        if total_holders >= 500 { score += 3.0; }
        else if total_holders < 50 { score -= 3.0; }
        score = score.max(0.0).min(25.0);

        HolderAnalysis {
            total_holders,
            sniper_count,
            sniper_percentage: sniper_pct,
            top10_concentration,
            gini_coefficient: gini,
            bundled_wallets_detected: bundled,
            developer_sold_percentage: dev_sold_pct,
            score,
            flags,
            signals,
        }
    }

    // --- 2. تحليل السيولة ---
    fn analyze_liquidity(&self, pairs: &[PairData]) -> LiquidityAnalysis {
        let best_pair = pairs.iter()
            .max_by(|a, b| {
                let a_liq = a.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
                let b_liq = b.liquidity.as_ref().and_then(|l| l.usd).unwrap_or(0.0);
                a_liq.partial_cmp(&b_liq).unwrap()
            });

        let total_usd = best_pair
            .and_then(|p| p.liquidity.as_ref())
            .and_then(|l| l.usd)
            .unwrap_or(0.0);

        // حساب تأثير الأسعار بناءً على السيولة (نموذج مبسط)
        let price_impact_5k = if total_usd > 0.0 { (5000.0 / total_usd) * 100.0 } else { 100.0 };
        let price_impact_10k = if total_usd > 0.0 { (10000.0 / total_usd) * 100.0 } else { 100.0 };
        let price_impact_25k = if total_usd > 0.0 { (25000.0 / total_usd) * 100.0 } else { 100.0 };

        // كشف بناء السيولة (نحدده بناءً على عمر الزوج والسيولة)
        let lp_burned = total_usd > 50000.0; // تقدير مبسط
        let lp_locked = total_usd > 25000.0;
        let independent_providers = pairs.len() as u32;

        let mut flags = vec![];
        let mut signals = vec![];

        if total_usd < 5000.0 {
            flags.push(format!("🔴 سيولة منخفضة جداً: ${:.0}", total_usd));
        }
        if price_impact_10k > 5.0 {
            flags.push(format!("🔴 انزلاق سعري عالٍ: {:.1}% لصفقة $10K", price_impact_10k));
        }

        if total_usd >= 50000.0 {
            signals.push(format!("💰 سيولة ضخمة: ${:.0}", total_usd));
        } else if total_usd >= 25000.0 {
            signals.push(format!("💰 سيولة جيدة: ${:.0}", total_usd));
        } else if total_usd >= 10000.0 {
            signals.push(format!("💧 سيولة متوسطة: ${:.0}", total_usd));
        }

        if price_impact_10k < 3.0 {
            signals.push(format!("✅ انزلاق منخفض: {:.1}% لصفقة $10K", price_impact_10k));
        }
        if pairs.len() >= 2 {
            signals.push(format!("✅ متوفر على {} منصات تداول", pairs.len()));
        }

        // حساب النقطة (max 20)
        let mut score = 0.0f64;
        match total_usd as u64 {
            500_000.. => score += 20.0,
            100_000.. => score += 16.0,
            50_000.. => score += 13.0,
            25_000.. => score += 10.0,
            10_000.. => score += 6.0,
            5_000.. => score += 3.0,
            _ => {}
        }
        if price_impact_10k < 3.0 { score = (score + 2.0).min(20.0); }
        if pairs.len() >= 3 { score = (score + 2.0).min(20.0); }

        LiquidityAnalysis {
            total_usd,
            lp_burned,
            lp_locked,
            lock_duration_months: if lp_locked { Some(6.0) } else { None },
            independent_providers,
            price_impact_5k,
            price_impact_10k,
            price_impact_25k,
            score,
            flags,
            signals,
        }
    }

    // --- 3. تحليل الحيتان الذكية ---
    async fn analyze_whales(&mut self, address: &str) -> WhaleAnalysis {
        let txns = self.get_recent_transactions(address).await;
        let smart_wallets = self.data.smart_wallets.clone();

        let mut smart_entered = 0u32;
        let mut large_buys_usd: Vec<f64> = vec![];
        let mut cold_storage_transfers = 0u32;
        let mut has_distribution = false;
        let mut signals = vec![];

        for txn in &txns {
            // كشف المحافظ الذكية
            if let Some(fee_payer) = &txn.fee_payer {
                if smart_wallets.contains(fee_payer) {
                    smart_entered += 1;
                }
            }
            // تحليل حجم التحويلات
            if let Some(transfers) = &txn.token_transfers {
                for tf in transfers {
                    if tf.token_amount > 1_000_000.0 {
                        cold_storage_transfers += 1;
                    }
                }
            }
        }

        // اكتشاف الشراء الكبير
        let mut buy_amounts = vec![];
        let mut sell_amounts = vec![];
        for txn in &txns {
            if let Some(transfers) = &txn.token_transfers {
                let total_in: f64 = transfers.iter()
                    .filter(|tf| tf.to_user_account.is_some())
                    .map(|tf| tf.token_amount)
                    .sum();
                let total_out: f64 = transfers.iter()
                    .filter(|tf| tf.from_user_account.is_some())
                    .map(|tf| tf.token_amount)
                    .sum();
                if total_in > total_out { buy_amounts.push(total_in); }
                else { sell_amounts.push(total_out); }
            }
        }

        let largest_buy = buy_amounts.iter().cloned().fold(0.0_f64, f64::max);
        let accumulation = buy_amounts.len() > sell_amounts.len() * 2;
        has_distribution = sell_amounts.len() > buy_amounts.len();

        if smart_entered >= 3 {
            signals.push(format!("🐋 {} محافظ ذكية دخلت!", smart_entered));
        }
        if accumulation && !has_distribution {
            signals.push("📈 نمط تجميع واضح بدون توزيع".to_string());
        }
        if cold_storage_transfers >= 2 {
            signals.push(format!("🏦 {} تحويل لتخزين بارد", cold_storage_transfers));
        }

        // حساب النقطة (max 20)
        let mut score = 0.0f64;
        if smart_entered >= 3 { score += 12.0; }
        else if smart_entered >= 1 { score += 6.0; }
        if accumulation && !has_distribution { score += 5.0; }
        if cold_storage_transfers >= 2 { score += 3.0; }
        if has_distribution { score -= 5.0; }
        score = score.max(0.0).min(20.0);

        WhaleAnalysis {
            smart_wallets_entered: smart_entered,
            accumulation_pattern: accumulation,
            distribution_signs: has_distribution,
            cold_storage_transfers,
            largest_single_buy_usd: largest_buy,
            score,
            signals,
        }
    }

    // --- 4. التحليل الفني ---
    fn analyze_technicals(&self, pairs: &[PairData]) -> TechnicalAnalysis {
        let best = pairs.first();
        let m5 = best.and_then(|p| p.price_change.as_ref()).and_then(|pc| pc.m5).unwrap_or(0.0);
        let h1 = best.and_then(|p| p.price_change.as_ref()).and_then(|pc| pc.h1).unwrap_or(0.0);
        let h6 = best.and_then(|p| p.price_change.as_ref()).and_then(|pc| pc.h6).unwrap_or(0.0);
        let h24 = best.and_then(|p| p.price_change.as_ref()).and_then(|pc| pc.h24).unwrap_or(0.0);
        let vol24 = best.and_then(|p| p.volume.as_ref()).and_then(|v| v.h24).unwrap_or(0.0);

        let (buys, sells) = best
            .and_then(|p| p.txns.as_ref())
            .and_then(|t| t.h1.as_ref())
            .map(|t| (t.buys.unwrap_or(0), t.sells.unwrap_or(0)))
            .unwrap_or((0, 0));
        let total_txns = buys + sells;
        let buy_ratio = if total_txns > 0 { buys as f64 / total_txns as f64 } else { 0.5 };

        // كشف الأنماط
        let pattern = if m5 > 20.0 && h1 > 50.0 {
            Some("🚀 Bull Flag - زخم قوي".to_string())
        } else if m5 > 5.0 && h1 > 20.0 && h6 > 50.0 {
            Some("📈 Ascending Triangle".to_string())
        } else if h1 < -10.0 && h6 > 20.0 {
            Some("🔄 Wyckoff - تجميع محتمل".to_string())
        } else {
            None
        };

        let mut signals = vec![];
        if m5 > 20.0 { signals.push(format!("⚡ {:.1}% خلال 5 دقائق!", m5)); }
        if h1 > 50.0 { signals.push(format!("🚀 {:.1}% خلال ساعة!", h1)); }
        if buy_ratio > 0.7 { signals.push(format!("📈 ضغط شراء: {:.0}% شراء", buy_ratio * 100.0)); }
        if total_txns > 100 { signals.push(format!("🔥 {} معاملة/ساعة", total_txns)); }
        if let Some(p) = &pattern { signals.push(p.clone()); }
        if vol24 > 500_000.0 { signals.push(format!("📊 حجم تداول: ${:.0}", vol24)); }

        // حساب النقطة (max 10)
        let mut score = 0.0f64;
        if m5 > 20.0 { score += 3.0; } else if m5 > 10.0 { score += 1.5; }
        if h1 > 50.0 { score += 3.0; } else if h1 > 20.0 { score += 1.5; }
        if buy_ratio > 0.7 { score += 2.0; }
        if total_txns > 100 { score += 2.0; }
        score = score.max(0.0).min(10.0);

        TechnicalAnalysis {
            momentum_5m: m5,
            momentum_1h: h1,
            momentum_6h: h6,
            momentum_24h: h24,
            volume_24h: vol24,
            buy_pressure_ratio: buy_ratio,
            pattern_detected: pattern,
            support_level: None,
            resistance_level: None,
            score,
            signals,
        }
    }

    // --- 5. أمان العقد ---
    async fn analyze_contract_security(&mut self, address: &str) -> ContractSecurity {
        let meta = self.get_token_metadata(address).await;
        let mut flags = vec![];
        let mut signals = vec![];

        let mint_revoked = meta.as_ref()
            .map(|m| m.mint_authority.is_none())
            .unwrap_or(false);
        let freeze_revoked = meta.as_ref()
            .map(|m| m.freeze_authority.is_none())
            .unwrap_or(false);

        if !mint_revoked {
            flags.push("🔴 صلاحية Mint غير ملغية - خطر سك عملات جديدة".to_string());
        } else {
            signals.push("✅ صلاحية Mint ملغية".to_string());
        }
        if !freeze_revoked {
            flags.push("⚠️ صلاحية Freeze نشطة - يمكن تجميد المحافظ".to_string());
        } else {
            signals.push("✅ صلاحية Freeze ملغية".to_string());
        }

        // حساب النقطة (max 10)
        let mut score = 0.0f64;
        if mint_revoked { score += 6.0; }
        if freeze_revoked { score += 4.0; }

        ContractSecurity {
            mint_authority_revoked: mint_revoked,
            freeze_authority_revoked: freeze_revoked,
            transfer_fee_percent: 0.0,
            honeypot_risk: false,
            score,
            flags,
            signals,
        }
    }

    // --- 6. التحليل الاجتماعي ---
    fn analyze_social(&self, token: &DexToken) -> SocialAnalysis {
        let desc = token.description.as_deref().unwrap_or("");
        let has_twitter = desc.contains("twitter") || desc.contains("x.com") || desc.contains("t.me");
        let has_telegram = desc.contains("t.me") || desc.contains("telegram");
        let social_hype = if has_twitter && has_telegram { 70.0 }
            else if has_twitter || has_telegram { 40.0 }
            else { 10.0 };

        let mut signals = vec![];
        if has_twitter { signals.push("🐦 تويتر موجود".to_string()); }
        if has_telegram { signals.push("📱 تيليجرام موجود".to_string()); }

        let score = if has_twitter && has_telegram { 10.0 }
            else if has_twitter || has_telegram { 6.0 }
            else { 2.0 };

        SocialAnalysis {
            has_twitter,
            has_telegram,
            social_hype_score: social_hype,
            score,
            signals,
        }
    }

    // --- 7. دورة حياة العملة ---
    fn analyze_lifecycle(&self, pairs: &[PairData]) -> TokenLifecycle {
        let now = Utc::now().timestamp();
        let created = pairs.first()
            .and_then(|p| p.pair_created_at)
            .unwrap_or(now);
        let age_minutes = (now - created) / 60;

        let phase = match age_minutes {
            0..=30 => LifecyclePhase::Launch,
            31..=180 => LifecyclePhase::FirstDip,
            181..=720 => LifecyclePhase::Accumulation,
            721..=2880 => LifecyclePhase::Breakout,
            _ => LifecyclePhase::Mature,
        };

        let timing_score = match &phase {
            LifecyclePhase::Accumulation => 10.0,
            LifecyclePhase::FirstDip => 7.0,
            LifecyclePhase::Breakout => 5.0,
            LifecyclePhase::Launch => 4.0,
            LifecyclePhase::Mature => 1.0,
        };

        let rr_ratio = match &phase {
            LifecyclePhase::Accumulation => 15.0,
            LifecyclePhase::FirstDip => 10.0,
            LifecyclePhase::Breakout => 5.0,
            _ => 3.0,
        };

        TokenLifecycle { age_minutes, phase, entry_timing_score: timing_score, risk_reward_ratio: rr_ratio }
    }

    // ============================================================
    // MASTER ANALYSIS
    // ============================================================

    async fn full_analyze(&mut self, token: &DexToken) -> Option<FullTokenAnalysis> {
        let addr = &token.token_address;
        let pairs = self.get_pairs(addr).await;
        if pairs.is_empty() { return None; }

        let holder = self.analyze_holders(addr, &pairs).await;
        // تخطى إذا كان هناك علامات حمراء قاتلة
        if holder.bundled_wallets_detected {
            println!("  ⏭ تخطي - محافظ مجمعة مكتشفة");
            return None;
        }

        let liquidity = self.analyze_liquidity(&pairs);
        if liquidity.total_usd < 5000.0 {
            println!("  ⏭ تخطي - سيولة منخفضة جداً");
            return None;
        }

        let whale = self.analyze_whales(addr).await;
        let technical = self.analyze_technicals(&pairs);
        let security = self.analyze_contract_security(addr).await;

        if !security.mint_authority_revoked {
            println!("  ⏭ تخطي - صلاحية Mint غير ملغية");
            return None;
        }

        let social = self.analyze_social(token);
        let lifecycle = self.analyze_lifecycle(&pairs);

        // النقطة الإجمالية
        let total = holder.score + liquidity.score + whale.score
            + technical.score + security.score + social.score;
        // إضافة توقيت الدخول (حتى 5 نقاط إضافية)
        let total = (total + lifecycle.entry_timing_score * 0.5).min(100.0);

        let min_score = if lifecycle.age_minutes < 360 {
            MIN_SCORE_NEW_TOKEN
        } else {
            MIN_SCORE_OLDER_TOKEN
        };

        if total < min_score {
            println!("  ⚪ نقاط غير كافية: {:.1} < {:.1}", total, min_score);
            return None;
        }

        // جمع العلامات الحمراء
        let mut all_flags = vec![];
        all_flags.extend(holder.flags.clone());
        all_flags.extend(liquidity.flags.clone());
        all_flags.extend(security.flags.clone());

        // أهم الإشارات
        let mut top_signals = vec![];
        top_signals.extend(holder.signals.clone());
        top_signals.extend(liquidity.signals.clone());
        top_signals.extend(whale.signals.clone());
        top_signals.extend(technical.signals.clone());
        top_signals.extend(security.signals.clone());
        top_signals.extend(social.signals.clone());
        top_signals.truncate(8);

        let confidence = match total as u32 {
            90.. => ConfidenceLevel::VeryHigh,
            85.. => ConfidenceLevel::High,
            80.. => ConfidenceLevel::Medium,
            _ => ConfidenceLevel::Low,
        };

        let alert_level = match total as u32 {
            90.. => AlertLevel::Legendary,
            85.. => AlertLevel::Golden,
            80.. => AlertLevel::Excellent,
            _ => AlertLevel::Normal,
        };

        let potential = match total as u32 {
            90.. => "50x-100x 🌟",
            85.. => "20x-50x 💎",
            80.. => "10x-20x 🔥",
            _ => "5x-10x ⭐",
        };

        let price = pairs.first()
            .and_then(|p| p.price_usd.as_ref())
            .and_then(|p| p.parse::<f64>().ok());

        let market_cap = pairs.iter()
            .filter_map(|p| p.market_cap)
            .reduce(f64::max);

        let dex_urls: Vec<String> = pairs.iter().map(|p| p.url.clone()).collect();

        Some(FullTokenAnalysis {
            token_address: addr.clone(),
            symbol: token.symbol.clone().unwrap_or("UNKNOWN".to_string()),
            name: token.name.clone().unwrap_or("Unknown".to_string()),
            image_url: token.image_url.clone(),
            price_usd: price,
            market_cap,
            dex_urls,
            total_score: total,
            confidence_level: confidence,
            potential_multiplier: potential.to_string(),
            holder_analysis: holder,
            liquidity_analysis: liquidity,
            whale_analysis: whale,
            technical_analysis: technical,
            contract_security: security,
            social_analysis: social,
            lifecycle,
            alert_level,
            all_red_flags: all_flags,
            top_signals,
        })
    }

    // ============================================================
    // MESSAGE FORMATTING
    // ============================================================

    fn format_alert(&self, a: &FullTokenAnalysis) -> String {
        let mut m = String::new();
        m.push_str(&format!("{}\n", a.alert_level.to_header()));
        m.push_str("═══════════════════════════════\n\n");

        m.push_str(&format!("**{}** `({})`\n", a.name, a.symbol));
        m.push_str(&format!("📍 `{}`\n\n", a.token_address));

        // لوحة النقاط
        m.push_str("📊 **لوحة التحليل:**\n");
        m.push_str(&format!("⭐ الإجمالي: **{:.1}/100**\n", a.total_score));
        m.push_str(&format!("👥 الحاملون: **{:.1}/25**\n", a.holder_analysis.score));
        m.push_str(&format!("🌊 السيولة: **{:.1}/20**\n", a.liquidity_analysis.score));
        m.push_str(&format!("🐋 الحيتان: **{:.1}/20**\n", a.whale_analysis.score));
        m.push_str(&format!("📈 التقني: **{:.1}/10**\n", a.technical_analysis.score));
        m.push_str(&format!("🛡️ الأمان: **{:.1}/10**\n", a.contract_security.score));
        m.push_str(&format!("🌐 الاجتماعي: **{:.1}/15**\n\n", a.social_analysis.score));

        // البيانات المالية
        m.push_str("💰 **البيانات المالية:**\n");
        if let Some(price) = a.price_usd {
            m.push_str(&format!("💵 السعر: **${:.8}**\n", price));
        }
        if let Some(mc) = a.market_cap {
            let mc_str = format_usd(mc);
            m.push_str(&format!("🏛️ Market Cap: **{}**\n", mc_str));
        }
        let liq_str = format_usd(a.liquidity_analysis.total_usd);
        m.push_str(&format!("🌊 السيولة: **{}**\n", liq_str));
        let vol_str = format_usd(a.technical_analysis.volume_24h);
        m.push_str(&format!("📊 الحجم 24h: **{}**\n\n", vol_str));

        // توقيت الدخول
        m.push_str("⏰ **توقيت الدخول:**\n");
        m.push_str(&format!("{}\n", a.lifecycle.phase.to_arabic()));
        m.push_str(&format!("⏱️ عمر العملة: **{} دقيقة**\n", a.lifecycle.age_minutes));
        m.push_str(&format!("📐 R/R Ratio: **1:{:.0}**\n\n", a.lifecycle.risk_reward_ratio));

        // المضاعف المحتمل
        m.push_str(&format!("🎯 **المضاعف المحتمل: {}**\n\n", a.potential_multiplier));

        // الإشارات
        m.push_str("✅ **أهم الإشارات:**\n");
        for signal in &a.top_signals {
            m.push_str(&format!("▫️ {}\n", signal));
        }

        // العلامات الحمراء (إن وجدت)
        if !a.all_red_flags.is_empty() {
            m.push_str("\n⚠️ **تحذيرات:**\n");
            for flag in a.all_red_flags.iter().take(3) {
                m.push_str(&format!("▪️ {}\n", flag));
            }
        }

        m.push_str("\n═══════════════════════════════\n");
        m.push_str("⚠️ **هذا تحليل آلي وليس نصيحة مالية**\n");
        m.push_str("🔍 قم ببحثك الخاص قبل الاستثمار\n");
        m
    }

    fn format_profit_message(&self, token: &TrackedToken, current: f64, pct: f64, milestone: u32) -> String {
        let (emoji, title) = match milestone {
            50 => ("🎉", "بداية موفقة"),
            100 => ("🚀💎", "مضاعفة رأس المال"),
            200 => ("🔥💰", "ربح استثنائي"),
            500 => ("⭐🏆", "أداء أسطوري"),
            1000 => ("👑💎", "عملة 10x الذهبية"),
            _ => ("🌟🚀", "ربح خرافي"),
        };
        let hours = Utc::now().signed_duration_since(token.discovery_time).num_hours();
        format!(
            "{} **{}!** {}\n═══════════════════════════════\n\n\
            💎 العملة: **{}** `({})`\n\
            📈 الربح: **+{:.1}%**\n\
            💰 سعر الاكتشاف: **${:.8}**\n\
            💰 السعر الحالي: **${:.8}**\n\
            ⏰ منذ الاكتشاف: **{} ساعة**\n\n\
            🎉 **مبروك لجميع المتابعين!**\n\
            🤖 تم اكتشاف هذه الفرصة بواسطة البوت",
            emoji, title, emoji,
            token.name, token.symbol,
            pct,
            token.initial_price,
            current,
            hours
        )
    }

    fn format_status(&self) -> String {
        let stats = &self.data.performance_stats;
        format!(
            "📊 **حالة البوت**\n═══════════════════════════════\n\
            🔍 العملات المرصودة: **{}**\n\
            📌 العملات المتتبعة: **{}**\n\
            📢 تنبيهات اليوم: **{}/{}**\n\
            🏆 إجمالي التنبيهات: **{}**\n\
            💹 عملات حققت 2x: **{}**\n\
            💹 عملات حققت 5x: **{}**\n\
            💹 عملات حققت 10x: **{}**\n\
            🥇 أفضل عملة: **{}** ({:.1}%)\n\
            🤖 حالة البوت: **{}**",
            self.data.seen_tokens.len(),
            self.data.tracked_tokens.len(),
            self.data.daily_alert_count, MAX_DAILY_ALERTS,
            stats.total_alerts_sent,
            stats.tokens_reached_2x,
            stats.tokens_reached_5x,
            stats.tokens_reached_10x,
            stats.best_token_symbol, stats.best_token_gain_percent,
            if self.is_paused { "⏸ متوقف مؤقتاً" } else { "▶️ يعمل" }
        )
    }

    // ============================================================
    // PROFIT TRACKING
    // ============================================================

    async fn check_profits(&mut self) {
        let addresses: Vec<String> = self.data.tracked_tokens.keys().cloned().collect();
        for addr in addresses {
            self.helius_limiter.wait_if_needed().await;
            let url = format!("https://api.dexscreener.com/latest/dex/tokens/{}", addr);
            let current_price = match self.client.get(&url).send().await {
                Ok(r) if r.status().is_success() => {
                    r.json::<PairResponse>().await.ok()
                        .and_then(|pr| pr.pairs)
                        .and_then(|ps| ps.into_iter().next())
                        .and_then(|p| p.price_usd)
                        .and_then(|p| p.parse::<f64>().ok())
                }
                _ => None,
            };

            if let Some(price) = current_price {
                // استخراج البيانات المطلوبة أولاً قبل أي استعارة
                let (initial_price, current_milestones, token_name, token_symbol, token_img) = {
                    let token = self.data.tracked_tokens.get_mut(&addr).unwrap();
                    if price > token.highest_price { token.highest_price = price; }
                    (
                        token.initial_price,
                        token.milestones_reached.clone(),
                        token.name.clone(),
                        token.symbol.clone(),
                        token.image_url.clone(),
                    )
                };

                let pct = (price - initial_price) / initial_price * 100.0;
                let milestones = [50u32, 100, 200, 500, 1000, 2000];

                // تحديد المراحل الجديدة التي وصلناها
                let new_milestones: Vec<u32> = milestones.iter()
                    .filter(|&&ms| pct >= ms as f64 && !current_milestones.contains(&ms))
                    .copied()
                    .collect();

                for ms in new_milestones {
                    // تسجيل المرحلة في البيانات
                    if let Some(token) = self.data.tracked_tokens.get_mut(&addr) {
                        token.milestones_reached.push(ms);
                    }

                    // بناء رسالة الربح يدوياً بدون استعارة TrackedToken
                    let (emoji, title) = match ms {
                        50   => ("🎉", "بداية موفقة"),
                        100  => ("🚀💎", "مضاعفة رأس المال"),
                        200  => ("🔥💰", "ربح استثنائي"),
                        500  => ("⭐🏆", "أداء أسطوري"),
                        1000 => ("👑💎", "عملة 10x الذهبية"),
                        _    => ("🌟🚀", "ربح خرافي"),
                    };
                    let hours = Utc::now()
                        .signed_duration_since(
                            self.data.tracked_tokens.get(&addr)
                                .map(|t| t.discovery_time)
                                .unwrap_or(Utc::now())
                        )
                        .num_hours();
                    let msg = format!(
                        "{} **{}!** {}\n═══════════════════════════════\n\n\
                        💎 العملة: **{}** `({})`\n\
                        📈 الربح: **+{:.1}%**\n\
                        💰 سعر الاكتشاف: **${:.8}**\n\
                        💰 السعر الحالي: **${:.8}**\n\
                        ⏰ منذ الاكتشاف: **{} ساعة**\n\n\
                        🎉 **مبروك لجميع المتابعين!**\n\
                        🤖 تم اكتشاف هذه الفرصة بواسطة البوت",
                        emoji, title, emoji,
                        token_name, token_symbol,
                        pct, initial_price, price, hours
                    );
                    let _ = self.send_message(&msg).await;

                    // تحديث الإحصاءات
                    if pct >= 100.0 { self.data.performance_stats.tokens_reached_2x += 1; }
                    if pct >= 400.0 { self.data.performance_stats.tokens_reached_5x += 1; }
                    if pct >= 900.0 { self.data.performance_stats.tokens_reached_10x += 1; }
                    if pct > self.data.performance_stats.best_token_gain_percent {
                        self.data.performance_stats.best_token_gain_percent = pct;
                        self.data.performance_stats.best_token_symbol = token_symbol.clone();
                    }
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    // ============================================================
    // MAIN LOOP
    // ============================================================

    async fn run(&mut self) {
        println!("🚀 محلل Solana المتقدم بدأ العمل...");
        self.load();
        let startup = format!(
            "🤖 **بوت Solana المتقدم بدأ!**\n\n\
            🎯 العتبة: {:.0} نقطة (جديد) / {:.0} (أقدم)\n\
            ⏱️ مسح كل {} ثانية\n\
            📢 حد التنبيهات: {} يومياً\n\
            🔍 أقصى عمر للعملة: {} ساعات",
            MIN_SCORE_NEW_TOKEN, MIN_SCORE_OLDER_TOKEN,
            SCAN_INTERVAL_SECS, MAX_DAILY_ALERTS, MAX_TOKEN_AGE_HOURS
        );
        if let Err(e) = self.send_message(&startup).await {
            println!("❌ خطأ في إرسال رسالة البداية: {}", e);
        }

        let mut scan_count = 0u64;
        let mut profit_timer = 0u64;

        loop {
            if !self.is_paused {
                self.reset_daily_count_if_needed();

                // فحص الأرباح كل 5 دقائق
                if profit_timer >= PROFIT_CHECK_INTERVAL_SECS {
                    self.check_profits().await;
                    profit_timer = 0;
                }

                // المسح الرئيسي
                match self.get_new_solana_tokens().await {
                    Ok(tokens) => {
                        println!("🔍 فحص {} عملة جديدة... (دورة {})", tokens.len(), scan_count);
                        for token in tokens {
                            let addr = token.token_address.clone();
                            if self.data.seen_tokens.contains_key(&addr) { continue; }
                            self.data.seen_tokens.insert(addr.clone(), Utc::now().to_rfc3339());

                            if self.data.daily_alert_count >= MAX_DAILY_ALERTS {
                                println!("  ⚠️ وصل حد التنبيهات اليومي");
                                break;
                            }

                            println!("  🆕 تحليل: {}", token.symbol.as_deref().unwrap_or("?"));
                            sleep(Duration::from_millis(300)).await;

                            if let Some(analysis) = self.full_analyze(&token).await {
                                println!("  🎯 فرصة! نقاط: {:.1}", analysis.total_score);

                                // إرسال الصورة إن وجدت
                                if let Some(img) = &analysis.image_url {
                                    let caption = format!("🔥 **{}** | {:.1}/100", analysis.symbol, analysis.total_score);
                                    self.send_photo(img, &caption).await;
                                    sleep(Duration::from_millis(500)).await;
                                }

                                let msg = self.format_alert(&analysis);
                                let dex_url = analysis.dex_urls.first().cloned().unwrap_or_default();
                                self.send_alert_with_buttons(&msg, &dex_url).await;

                                // تتبع العملة
                                if let Some(price) = analysis.price_usd {
                                    self.data.tracked_tokens.insert(addr.clone(), TrackedToken {
                                        token_address: addr.clone(),
                                        symbol: analysis.symbol.clone(),
                                        name: analysis.name.clone(),
                                        image_url: analysis.image_url.clone(),
                                        initial_price: price,
                                        highest_price: price,
                                        discovery_time: Utc::now(),
                                        milestones_reached: vec![],
                                    });
                                }

                                self.data.daily_alert_count += 1;
                                self.data.performance_stats.total_alerts_sent += 1;
                                sleep(Duration::from_secs(3)).await;
                            }
                            sleep(Duration::from_millis(500)).await;
                        }
                    }
                    Err(e) => println!("❌ خطأ في المسح: {}", e),
                }

                // حفظ دوري
                if Utc::now().signed_duration_since(self.last_save).num_minutes() >= SAVE_INTERVAL_MINS {
                    let _ = self.save();
                    self.last_save = Utc::now();
                }
            }

            scan_count += 1;
            profit_timer += SCAN_INTERVAL_SECS;
            sleep(Duration::from_secs(SCAN_INTERVAL_SECS)).await;
        }
    }
}

// ============================================================
// HELPER FUNCTIONS
// ============================================================

fn calculate_gini(amounts: &[f64]) -> f64 {
    if amounts.is_empty() { return 0.0; }
    let mut sorted = amounts.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = sorted.len() as f64;
    let total: f64 = sorted.iter().sum();
    if total == 0.0 { return 0.0; }
    let weighted_sum: f64 = sorted.iter().enumerate()
        .map(|(i, &x)| (2.0 * (i + 1) as f64 - n - 1.0) * x)
        .sum();
    weighted_sum / (n * total)
}

fn format_usd(amount: f64) -> String {
    if amount >= 1_000_000.0 { format!("${:.2}M", amount / 1_000_000.0) }
    else if amount >= 1_000.0 { format!("${:.1}K", amount / 1_000.0) }
    else { format!("${:.0}", amount) }
}

// ============================================================
// ENTRY POINT
// ============================================================

#[tokio::main]
async fn main() {
    println!("═══════════════════════════════════════════════════");
    println!("🌟 محلل Solana المتقدم - الإصدار 2.0 الاحترافي");
    println!("═══════════════════════════════════════════════════");
    println!("📋 نظام التسجيل المتكامل:");
    println!("  👥 الحاملون:     25 نقطة");
    println!("  🌊 السيولة:      20 نقطة");
    println!("  🐋 الحيتان:      20 نقطة");
    println!("  📈 التقني:       10 نقطة");
    println!("  🛡️ الأمان:       10 نقطة");
    println!("  🌐 الاجتماعي:    15 نقطة");
    println!("═══════════════════════════════════════════════════\n");

    if HELIUS_API_KEY == "YOUR_HELIUS_KEY_HERE" || TELEGRAM_BOT_TOKEN == "YOUR_TELEGRAM_TOKEN_HERE" {
        eprintln!("❌ خطأ: ضع API Keys الخاصة بك في قسم CONFIG أعلى الملف!");
        std::process::exit(1);
    }

    let mut bot = SolanaBot::new();
    bot.run().await;
}
