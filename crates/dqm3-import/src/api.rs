//! MediaWiki API client for wikidragonquest.fr: polite (throttled,
//! retried with backoff on overload), with a local on-disk cache.

use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

pub const WIKI_ROOT: &str = "https://wikidragonquest.fr";

/// A wiki page as returned by the API (after redirects).
#[derive(Debug, Clone, PartialEq)]
pub struct WikiPage {
    pub title: String,
    pub content: String,
    /// Title of the `en` interlanguage link, if any.
    pub en_link: Option<String>,
    pub url: String,
}

// ---------- Response shapes (formatversion=2) ----------

#[derive(Deserialize)]
struct RawResponse {
    error: Option<serde_json::Value>,
    query: Option<RawQuery>,
    /// Parameters to send back to get the rest of the result, if any.
    #[serde(rename = "continue")]
    continue_params: Option<BTreeMap<String, String>>,
}

#[derive(Deserialize)]
struct RawQuery {
    #[serde(default)]
    normalized: Vec<RawAlias>,
    #[serde(default)]
    redirects: Vec<RawAlias>,
    #[serde(default)]
    pages: Vec<RawPage>,
}

#[derive(Deserialize)]
struct RawAlias {
    from: String,
    to: String,
}

#[derive(Deserialize)]
struct RawPage {
    title: String,
    #[serde(default)]
    missing: bool,
    canonicalurl: Option<String>,
    #[serde(default)]
    langlinks: Vec<RawLangLink>,
    #[serde(default)]
    revisions: Vec<RawRevision>,
}

#[derive(Deserialize)]
struct RawLangLink {
    lang: String,
    title: String,
}

#[derive(Deserialize)]
struct RawRevision {
    slots: RawSlots,
}

#[derive(Deserialize)]
struct RawSlots {
    main: RawSlot,
}

#[derive(Deserialize)]
struct RawSlot {
    content: String,
}

/// One parsed API response.
#[derive(Debug, Default, PartialEq)]
pub struct Response {
    /// Existing pages only (missing ones are dropped).
    pub pages: Vec<WikiPage>,
    /// `from -> to` for title normalisation and redirects.
    pub aliases: Vec<(String, String)>,
    pub next: Option<BTreeMap<String, String>>,
}

impl RawPage {
    /// Consumes the raw page (`self` by value: strings are moved, not copied).
    fn into_page(self) -> WikiPage {
        let content = self
            .revisions
            .into_iter()
            .next()
            .map(|r| r.slots.main.content)
            .unwrap_or_default();
        let en_link = self
            .langlinks
            .into_iter()
            .find(|l| l.lang == "en")
            .map(|l| l.title);
        let url = self.canonicalurl.unwrap_or_else(|| page_url(&self.title));
        WikiPage {
            title: self.title,
            content,
            en_link,
            url,
        }
    }
}

/// Fallback URL of a page from its title.
pub fn page_url(title: &str) -> String {
    format!("{WIKI_ROOT}/index.php?title={}", title.replace(' ', "_"))
}

/// Parses one API response body.
pub fn parse_response(body: &str) -> serde_json::Result<Response> {
    let raw: RawResponse = serde_json::from_str(body)?;
    if let Some(error) = raw.error {
        // MediaWiki reports some failures as HTTP 200 + {"error": ...}.
        return Err(serde::de::Error::custom(format!(
            "MediaWiki API error: {error}"
        )));
    }
    let query = raw.query.unwrap_or(RawQuery {
        normalized: Vec::new(),
        redirects: Vec::new(),
        pages: Vec::new(),
    });
    Ok(Response {
        pages: query
            .pages
            .into_iter()
            .filter(|p| !p.missing)
            .map(RawPage::into_page)
            .collect(),
        aliases: query
            .normalized
            .into_iter()
            .chain(query.redirects)
            .map(|a| (a.from, a.to))
            .collect(),
        next: raw.continue_params,
    })
}

/// Pages accumulated over several responses, with title aliases.
#[derive(Debug, Default)]
pub struct PageSet {
    pages: HashMap<String, WikiPage>,
    aliases: HashMap<String, String>,
}

impl PageSet {
    /// Adds a response. A page already seen without content (MediaWiki may
    /// split a large result) is completed rather than duplicated.
    pub fn add(&mut self, response: Response) {
        self.aliases.extend(response.aliases);
        for page in response.pages {
            match self.pages.get_mut(&page.title) {
                None => {
                    self.pages.insert(page.title.clone(), page);
                }
                Some(existing) => {
                    if existing.content.is_empty() {
                        existing.content = page.content;
                    }
                    if existing.en_link.is_none() {
                        existing.en_link = page.en_link;
                    }
                }
            }
        }
    }

    /// Finds the page for a requested title, following normalisation and
    /// redirects. `None` if the page does not exist.
    pub fn resolve(&self, title: &str) -> Option<&WikiPage> {
        let mut current = title;
        // Bounded: protects against a redirect loop.
        for _ in 0..5 {
            if let Some(page) = self.pages.get(current) {
                return Some(page);
            }
            current = self.aliases.get(current)?;
        }
        None
    }
}

/// Statuses worth retrying later: overload / temporary server errors.
pub fn is_retryable(status: u16) -> bool {
    status == 429 || (500..600).contains(&status)
}

/// Stable cache file name for a set of query parameters (FNV-1a 64-bit).
/// `std`'s `DefaultHasher` is not guaranteed stable across Rust versions,
/// so a tiny explicit hash is used instead.
pub fn cache_key(params: &BTreeMap<String, String>) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    // BTreeMap iterates in key order, so the same params give the same hash.
    for (key, value) in params {
        for byte in key.bytes().chain(*b"=").chain(value.bytes()).chain(*b"\n") {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(PRIME);
        }
    }
    format!("{hash:016x}.json")
}

// ---------- Network client ----------

const API_URL: &str = "https://wikidragonquest.fr/api.php";
/// Explicit, generic User-Agent (no personal data).
const USER_AGENT: &str = "dqm3-import/0.1 (non-commercial DQM3 fan tool; manual one-shot import; cached; slow on purpose)";
/// Politeness: at least this long between two requests (shared hosting).
const MIN_INTERVAL: Duration = Duration::from_secs(10);
/// Waits before each retry when the server is overloaded.
const BACKOFF_SECS: [u64; 3] = [30, 60, 120];
/// Kept small on purpose: a 50-page query (MediaWiki's maximum) was too
/// heavy for this shared host and got 508/500 errors.
const TITLES_PER_REQUEST: usize = 10;

/// How long to wait after `failed_attempts` failures, or `None` to give up
/// (no pointless wait after the last attempt).
pub fn retry_delay(failed_attempts: usize) -> Option<Duration> {
    let index = failed_attempts.checked_sub(1)?;
    BACKOFF_SECS.get(index).copied().map(Duration::from_secs)
}

pub type Error = Box<dyn std::error::Error>;

/// Throttled, cached MediaWiki client. Only `fetch_pages` hits the network,
/// and only for requests that are not already in the cache.
pub struct Client {
    agent: ureq::Agent,
    cache_dir: PathBuf,
    last_request: Option<Instant>,
}

impl Client {
    pub fn new(cache_dir: PathBuf) -> Result<Self, Error> {
        fs::create_dir_all(&cache_dir)?;
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .user_agent(USER_AGENT)
            .timeout_global(Some(Duration::from_secs(60)))
            .build()
            .into();
        Ok(Self {
            agent,
            cache_dir,
            last_request: None,
        })
    }

    /// Content, English link and URL of the given pages, redirects followed.
    /// `&[String]`: a borrowed slice, the caller keeps ownership of titles.
    pub fn fetch_pages(&mut self, titles: &[String]) -> Result<PageSet, Error> {
        let mut set = PageSet::default();
        let batches = titles.chunks(TITLES_PER_REQUEST);
        let total = batches.len();
        for (i, chunk) in batches.enumerate() {
            println!("Batch {}/{total} ({} titles)", i + 1, chunk.len());
            let base: BTreeMap<String, String> = [
                ("action", "query"),
                ("format", "json"),
                ("formatversion", "2"),
                ("redirects", "1"),
                ("prop", "revisions|langlinks|info"),
                ("rvprop", "content"),
                ("rvslots", "main"),
                ("lllang", "en"),
                ("lllimit", "max"),
                ("inprop", "url"),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .chain([("titles".to_string(), chunk.join("|"))])
            .collect();

            let mut params = base.clone();
            loop {
                let response = self.fetch(&params)?;
                let next = response.next.clone();
                set.add(response);
                match next {
                    Some(next) => {
                        params = base.clone();
                        params.extend(next);
                    }
                    None => break,
                }
            }
        }
        Ok(set)
    }

    /// One response, from the cache if present, otherwise from the network
    /// (with retries). Only valid responses are cached.
    fn fetch(&mut self, params: &BTreeMap<String, String>) -> Result<Response, Error> {
        let path = self.cache_dir.join(cache_key(params));
        if path.exists() {
            println!("  cache: {}", path.display());
            return Ok(parse_response(&fs::read_to_string(&path)?)?);
        }

        let mut failures = 0;
        loop {
            match self.request(params) {
                Ok(body) => match parse_response(&body) {
                    Ok(response) => {
                        fs::write(&path, &body)?;
                        return Ok(response);
                    }
                    // e.g. DBConnectionError returned with HTTP 200.
                    Err(e) => eprintln!("  attempt {}: invalid response ({e})", failures + 1),
                },
                Err(ureq::Error::StatusCode(code)) if is_retryable(code) => {
                    eprintln!(
                        "  attempt {}: HTTP {code} (server overloaded)",
                        failures + 1
                    );
                }
                Err(e) => return Err(e.into()),
            }
            failures += 1;
            let Some(wait) = retry_delay(failures) else {
                return Err("wiki unavailable after several attempts; progress is \
                            cached, run again later"
                    .into());
            };
            eprintln!("  waiting {} s before retrying...", wait.as_secs());
            thread::sleep(wait);
        }
    }

    /// A single throttled GET to the API.
    fn request(&mut self, params: &BTreeMap<String, String>) -> Result<String, ureq::Error> {
        if let Some(last) = self.last_request {
            let elapsed = last.elapsed();
            if elapsed < MIN_INTERVAL {
                let wait = MIN_INTERVAL - elapsed;
                println!("  waiting {} s (politeness)", wait.as_secs());
                thread::sleep(wait);
            }
        }
        self.last_request = Some(Instant::now());
        println!("  GET {API_URL}");

        let mut request = self.agent.get(API_URL);
        for (key, value) in params {
            request = request.query(key, value);
        }
        request.call()?.body_mut().read_to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RESPONSE: &str = r#"{
        "batchcomplete": true,
        "continue": {"rvcontinue": "123|456", "continue": "||"},
        "query": {
            "normalized": [{"fromencoded": false, "from": "Espoir_de_Flamme", "to": "Espoir de Flamme"}],
            "redirects": [{"from": "Espoir de Flamme", "to": "Apprenti de Flamme"}],
            "pages": [
                {"pageid": 1, "ns": 0, "title": "Apprenti de Flamme",
                 "canonicalurl": "https://wikidragonquest.fr/Apprenti_de_Flamme",
                 "langlinks": [{"lang": "en", "title": "Frizz Afficionado"}],
                 "revisions": [{"slots": {"main": {"contentmodel": "wikitext", "content": "texte"}}}]},
                {"ns": 0, "title": "Page absente", "missing": true}
            ]
        }
    }"#;

    #[test]
    fn response_gives_pages_aliases_and_continuation() {
        let response = parse_response(RESPONSE).expect("valid fixture");
        assert_eq!(
            response.pages,
            vec![WikiPage {
                title: "Apprenti de Flamme".into(),
                content: "texte".into(),
                en_link: Some("Frizz Afficionado".into()),
                url: "https://wikidragonquest.fr/Apprenti_de_Flamme".into(),
            }]
        );
        assert_eq!(response.aliases.len(), 2);
        assert_eq!(
            response
                .next
                .and_then(|n| n.get("rvcontinue").cloned())
                .as_deref(),
            Some("123|456")
        );
    }

    #[test]
    fn redirect_and_normalisation_are_followed() {
        let mut set = PageSet::default();
        set.add(parse_response(RESPONSE).expect("valid fixture"));
        let page = set.resolve("Espoir_de_Flamme").expect("redirected page");
        assert_eq!(page.title, "Apprenti de Flamme");
        assert!(set.resolve("Apprenti de Flamme").is_some());
    }

    #[test]
    fn missing_page_resolves_to_none() {
        let mut set = PageSet::default();
        set.add(parse_response(RESPONSE).expect("valid fixture"));
        assert_eq!(set.resolve("Page absente"), None);
        assert_eq!(set.resolve("Jamais demandée"), None);
    }

    #[test]
    fn page_split_across_responses_is_completed() {
        let page = |content: &str, en: Option<&str>| WikiPage {
            title: "Assassin".into(),
            content: content.into(),
            en_link: en.map(Into::into),
            url: String::new(),
        };
        let mut set = PageSet::default();
        set.add(Response {
            pages: vec![page("", Some("Assassin"))],
            ..Response::default()
        });
        set.add(Response {
            pages: vec![page("texte", None)],
            ..Response::default()
        });
        assert_eq!(
            set.resolve("Assassin"),
            Some(&page("texte", Some("Assassin")))
        );
    }

    #[test]
    fn last_response_has_no_continuation() {
        let response =
            parse_response(r#"{"batchcomplete": true, "query": {"pages": []}}"#).expect("valid");
        assert_eq!(response.next, None);
    }

    #[test]
    fn api_error_body_is_an_error() {
        // MediaWiki reports DB failures as HTTP 200 + {"error": ...}.
        assert!(
            parse_response(r#"{"error": {"code": "internal_api_error_DBConnectionError"}}"#)
                .is_err()
        );
    }

    #[test]
    fn retry_waits_grow_then_give_up_without_final_wait() {
        let secs = |n| retry_delay(n).map(|d| d.as_secs());
        assert_eq!(secs(1), Some(30));
        assert_eq!(secs(2), Some(60));
        assert_eq!(secs(3), Some(120));
        assert_eq!(secs(4), None, "4th failure: stop at once");
    }

    #[test]
    fn overload_statuses_are_retryable() {
        assert!(is_retryable(508));
        assert!(is_retryable(503));
        assert!(is_retryable(429));
        assert!(!is_retryable(404));
        assert!(!is_retryable(200));
    }

    #[test]
    fn cache_key_is_stable_and_depends_on_params() {
        let mut a = BTreeMap::new();
        a.insert("action".to_string(), "query".to_string());
        let mut b = a.clone();
        b.insert("rvcontinue".to_string(), "x".to_string());

        assert_eq!(cache_key(&a), cache_key(&a.clone()));
        assert_ne!(cache_key(&a), cache_key(&b));
        assert_eq!(cache_key(&a).len(), 16 + ".json".len());
    }
}
