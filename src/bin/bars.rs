//! Find pubs & bars in Kraków and their walking distance to the nearest tram stops.
//!
//! Pipeline (two cached stages):
//!   Stage 1 — bar list (cached in `bars.csv`):
//!     1. Load tram stops from a GTFS `stops.txt` (only coordinates are needed).
//!     2. Query the Google Places "Nearby Search" API around each stop to build a
//!        deduplicated list of bars/pubs reachable from the tram network, and
//!        write it to `--bars`. On later runs this file is loaded as-is and the
//!        (expensive, rate-limited) Places step is skipped — pass `--refresh` to
//!        force a re-fetch.
//!   Stage 2 — join to stops (written to `bars_stops.csv`):
//!     3. For every bar, pick the closest stops offline via the haversine formula.
//!     4. Ask the Google Distance Matrix API for the real by-foot distance/duration
//!        from the bar to those stops (`mode=walking`).
//!     5. Write one CSV row per (bar, stop) pair.
//!
//! Usage:
//!   GOOGLE_MAPS_API_KEY=... cargo run --bin bars -- \
//!       [--stops data/GTFS_KRK_T/stops.txt] \
//!       [--bars bars.csv] \     # cached bar list (fetched if missing)
//!       [--out bars_stops.csv] \
//!       [--radius 500] \        # nearby-search radius around each stop, metres
//!       [--nearest 5] \         # how many stops to keep per bar
//!       [--limit N] \           # cap number of bars (cost control / testing)
//!       [--refresh] \           # re-fetch bars even if the cache exists
//!       [--no-walking]          # skip Distance Matrix, straight-line only (free)

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const PLACES_URL: &str = "https://maps.googleapis.com/maps/api/place/nearbysearch/json";
const MATRIX_URL: &str = "https://maps.googleapis.com/maps/api/distancematrix/json";

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

struct Args {
    stops: PathBuf,
    bars: PathBuf,
    out: PathBuf,
    radius: u32,
    nearest: usize,
    limit: Option<usize>,
    refresh: bool,
    walking: bool,
}

impl Args {
    fn parse() -> Result<Self> {
        let mut args = Args {
            stops: PathBuf::from("data/GTFS_KRK_T/stops.txt"),
            bars: PathBuf::from("bars.csv"),
            out: PathBuf::from("bars_stops.csv"),
            radius: 500,
            nearest: 5,
            limit: None,
            refresh: false,
            walking: true,
        };
        let mut it = std::env::args().skip(1);
        while let Some(flag) = it.next() {
            match flag.as_str() {
                "--stops" => args.stops = next_val(&mut it, &flag)?.into(),
                "--bars" => args.bars = next_val(&mut it, &flag)?.into(),
                "--out" => args.out = next_val(&mut it, &flag)?.into(),
                "--radius" => args.radius = next_val(&mut it, &flag)?.parse()?,
                "--nearest" => args.nearest = next_val(&mut it, &flag)?.parse()?,
                "--limit" => args.limit = Some(next_val(&mut it, &flag)?.parse()?),
                "--refresh" => args.refresh = true,
                "--no-walking" => args.walking = false,
                "-h" | "--help" => {
                    print_help();
                    std::process::exit(0);
                }
                other => bail!("unknown argument: {other} (try --help)"),
            }
        }
        Ok(args)
    }
}

fn next_val(it: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    it.next().with_context(|| format!("missing value for {flag}"))
}

fn print_help() {
    eprintln!(
        "bars — find pubs/bars near Kraków tram stops and their walking distance\n\n\
         Usage: GOOGLE_MAPS_API_KEY=... cargo run --bin bars -- [OPTIONS]\n\n\
         Options:\n\
         \x20 --stops <path>    GTFS stops.txt (default data/GTFS_KRK_T/stops.txt)\n\
         \x20 --bars <path>     Cached bar list; fetched if missing (default bars.csv)\n\
         \x20 --out <path>      Output CSV (default bars_stops.csv)\n\
         \x20 --radius <m>      Nearby-search radius around each stop (default 500)\n\
         \x20 --nearest <n>     Stops kept per bar (default 5)\n\
         \x20 --limit <n>       Process at most n bars (cost control)\n\
         \x20 --refresh         Re-fetch bars even if the cache exists\n\
         \x20 --no-walking      Skip Distance Matrix; straight-line distance only"
    );
}

// ---------------------------------------------------------------------------
// Stops
// ---------------------------------------------------------------------------

/// Minimal view of a GTFS stop — only what this tool needs.
#[derive(Debug, Deserialize)]
struct RawStop {
    stop_id: String,
    #[serde(default)]
    stop_code: String,
    #[serde(default)]
    stop_name: String,
    #[serde(default)]
    stop_lat: Option<f64>,
    #[serde(default)]
    stop_lon: Option<f64>,
}

#[derive(Debug, Clone)]
struct Stop {
    id: String,
    code: String,
    name: String,
    lat: f64,
    lon: f64,
}

fn load_stops(path: &PathBuf) -> Result<Vec<Stop>> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_path(path)
        .with_context(|| format!("opening {}", path.display()))?;
    let mut stops = Vec::new();
    for row in rdr.deserialize::<RawStop>() {
        let raw = match row {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Warning: skipping stop row – {e}");
                continue;
            }
        };
        match (raw.stop_lat, raw.stop_lon) {
            (Some(lat), Some(lon)) => stops.push(Stop {
                id: raw.stop_id,
                code: raw.stop_code,
                name: raw.stop_name,
                lat,
                lon,
            }),
            _ => eprintln!("Warning: stop {} has no coordinates, skipping", raw.stop_id),
        }
    }
    if stops.is_empty() {
        bail!("no stops with coordinates loaded from {}", path.display());
    }
    Ok(stops)
}

/// Great-circle distance in metres.
fn haversine_m(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6_371_000.0; // Earth radius, metres
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let a = (dlat / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * R * a.sqrt().asin()
}

// ---------------------------------------------------------------------------
// Google Places — Nearby Search
// ---------------------------------------------------------------------------

/// A bar/pub. Also the row schema of the cached `bars.csv`.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Bar {
    place_id: String,
    name: String,
    lat: f64,
    lon: f64,
    vicinity: String,
}

fn load_bars_csv(path: &PathBuf) -> Result<Vec<Bar>> {
    let mut rdr = csv::Reader::from_path(path)
        .with_context(|| format!("opening bar cache {}", path.display()))?;
    let mut bars = Vec::new();
    for row in rdr.deserialize::<Bar>() {
        match row {
            Ok(b) => bars.push(b),
            Err(e) => eprintln!("Warning: skipping bar row – {e}"),
        }
    }
    Ok(bars)
}

fn save_bars_csv(path: &PathBuf, bars: &[Bar]) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)
        .with_context(|| format!("writing bar cache {}", path.display()))?;
    for bar in bars {
        wtr.serialize(bar)?;
    }
    wtr.flush()?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct PlacesResponse {
    #[serde(default)]
    results: Vec<PlaceResult>,
    status: String,
    #[serde(default)]
    next_page_token: Option<String>,
    #[serde(default)]
    error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlaceResult {
    place_id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    vicinity: String,
    geometry: Geometry,
}

#[derive(Debug, Deserialize)]
struct Geometry {
    location: LatLng,
}

#[derive(Debug, Deserialize)]
struct LatLng {
    lat: f64,
    lng: f64,
}

/// Search `type=bar` around a single point, following pagination, into `out`.
fn search_around(
    client: &reqwest::blocking::Client,
    key: &str,
    lat: f64,
    lon: f64,
    radius: u32,
    out: &mut HashMap<String, Bar>,
) -> Result<()> {
    let mut page_token: Option<String> = None;
    loop {
        // A fresh page_token needs a moment before Google accepts it.
        if page_token.is_some() {
            std::thread::sleep(Duration::from_millis(2000));
        }

        let mut req = client.get(PLACES_URL).query(&[("key", key)]);
        req = match &page_token {
            Some(token) => req.query(&[("pagetoken", token.as_str())]),
            None => req.query(&[
                ("location", format!("{lat},{lon}").as_str()),
                ("radius", radius.to_string().as_str()),
                ("type", "bar"),
            ]),
        };

        let resp: PlacesResponse = req.send()?.error_for_status()?.json()?;
        match resp.status.as_str() {
            "OK" | "ZERO_RESULTS" => {}
            "OVER_QUERY_LIMIT" => {
                // Back off once and retry the same request.
                eprintln!("Warning: OVER_QUERY_LIMIT, backing off 3s");
                std::thread::sleep(Duration::from_secs(3));
                continue;
            }
            other => bail!(
                "Places API error: {other} ({})",
                resp.error_message.unwrap_or_default()
            ),
        }

        for p in resp.results {
            out.entry(p.place_id.clone()).or_insert(Bar {
                place_id: p.place_id,
                name: p.name,
                lat: p.geometry.location.lat,
                lon: p.geometry.location.lng,
                vicinity: p.vicinity,
            });
        }

        match resp.next_page_token {
            Some(t) => page_token = Some(t),
            None => break,
        }
    }
    Ok(())
}

fn fetch_bars(
    client: &reqwest::blocking::Client,
    key: &str,
    stops: &[Stop],
    radius: u32,
) -> Result<Vec<Bar>> {
    let mut bars: HashMap<String, Bar> = HashMap::new();
    let total = stops.len();
    for (i, stop) in stops.iter().enumerate() {
        if let Err(e) = search_around(client, key, stop.lat, stop.lon, radius, &mut bars) {
            eprintln!("Warning: search near stop {} failed: {e}", stop.id);
        }
        if (i + 1) % 20 == 0 || i + 1 == total {
            eprintln!("  searched {}/{total} stops, {} unique bars so far", i + 1, bars.len());
        }
    }
    Ok(bars.into_values().collect())
}

// ---------------------------------------------------------------------------
// Google Distance Matrix — walking
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct MatrixResponse {
    status: String,
    #[serde(default)]
    rows: Vec<MatrixRow>,
    #[serde(default)]
    error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MatrixRow {
    #[serde(default)]
    elements: Vec<MatrixElement>,
}

#[derive(Debug, Deserialize)]
struct MatrixElement {
    status: String,
    #[serde(default)]
    distance: Option<ValueText>,
    #[serde(default)]
    duration: Option<ValueText>,
}

#[derive(Debug, Deserialize)]
struct ValueText {
    value: f64,
}

/// Walking distance (m) and duration (s) from `origin` to each destination.
/// `None` per destination when Google can't route there.
fn walking_distances(
    client: &reqwest::blocking::Client,
    key: &str,
    origin: (f64, f64),
    dests: &[(f64, f64)],
) -> Result<Vec<Option<(f64, f64)>>> {
    let dest_param = dests
        .iter()
        .map(|(lat, lon)| format!("{lat},{lon}"))
        .collect::<Vec<_>>()
        .join("|");

    let resp: MatrixResponse = client
        .get(MATRIX_URL)
        .query(&[
            ("origins", format!("{},{}", origin.0, origin.1).as_str()),
            ("destinations", dest_param.as_str()),
            ("mode", "walking"),
            ("units", "metric"),
            ("key", key),
        ])
        .send()?
        .error_for_status()?
        .json()?;

    if resp.status != "OK" {
        bail!(
            "Distance Matrix error: {} ({})",
            resp.status,
            resp.error_message.unwrap_or_default()
        );
    }

    let row = resp.rows.into_iter().next().context("empty matrix response")?;
    Ok(row
        .elements
        .into_iter()
        .map(|el| match (el.status.as_str(), el.distance, el.duration) {
            ("OK", Some(d), Some(t)) => Some((d.value, t.value)),
            _ => None,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct OutRow<'a> {
    bar_place_id: &'a str,
    bar_name: &'a str,
    bar_vicinity: &'a str,
    bar_lat: f64,
    bar_lon: f64,
    rank: usize,
    stop_id: &'a str,
    stop_code: &'a str,
    stop_name: &'a str,
    stop_lat: f64,
    stop_lon: f64,
    straight_line_m: u64,
    walk_distance_m: Option<u64>,
    walk_duration_s: Option<u64>,
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

/// The key is only needed when we actually call Google — fetching bars or
/// computing walking distances. A cached, straight-line-only run needs no key.
fn require_key(key: &Option<String>) -> Result<&str> {
    key.as_deref().context(
        "GOOGLE_MAPS_API_KEY is not set. Create a key with the Places API and \
         Distance Matrix API enabled, then export it before running.",
    )
}

fn main() -> Result<()> {
    let args = Args::parse()?;
    let key = std::env::var("GOOGLE_MAPS_API_KEY").ok();

    let stops = load_stops(&args.stops)?;
    eprintln!("Loaded {} tram stops with coordinates", stops.len());

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;

    // Stage 1: bars — load from cache if present, otherwise fetch and cache.
    let mut bars = if args.bars.exists() && !args.refresh {
        let bars = load_bars_csv(&args.bars)?;
        eprintln!("Loaded {} bars from cache {}", bars.len(), args.bars.display());
        bars
    } else {
        eprintln!(
            "Fetching bars via Places Nearby Search ({}m radius around each stop)…",
            args.radius
        );
        let bars = fetch_bars(&client, require_key(&key)?, &stops, args.radius)?;
        save_bars_csv(&args.bars, &bars)?;
        eprintln!("Cached {} bars to {}", bars.len(), args.bars.display());
        bars
    };
    bars.sort_by(|a, b| a.name.cmp(&b.name)); // stable, reproducible order
    if let Some(limit) = args.limit {
        bars.truncate(limit);
    }
    eprintln!("Processing {} bars/pubs", bars.len());

    let mut wtr = csv::Writer::from_path(&args.out)
        .with_context(|| format!("creating {}", args.out.display()))?;

    for (bi, bar) in bars.iter().enumerate() {
        // 5 (or --nearest) closest stops by straight-line distance.
        let mut ranked: Vec<(&Stop, f64)> = stops
            .iter()
            .map(|s| (s, haversine_m(bar.lat, bar.lon, s.lat, s.lon)))
            .collect();
        ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        ranked.truncate(args.nearest);

        // Real walking distance for those stops, if enabled.
        let walk: Vec<Option<(f64, f64)>> = if args.walking {
            let dests: Vec<(f64, f64)> = ranked.iter().map(|(s, _)| (s.lat, s.lon)).collect();
            match walking_distances(&client, require_key(&key)?, (bar.lat, bar.lon), &dests) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Warning: walking distances for '{}' failed: {e}", bar.name);
                    vec![None; ranked.len()]
                }
            }
        } else {
            vec![None; ranked.len()]
        };

        for (rank, ((stop, straight), w)) in ranked.iter().zip(walk).enumerate() {
            wtr.serialize(OutRow {
                bar_place_id: &bar.place_id,
                bar_name: &bar.name,
                bar_vicinity: &bar.vicinity,
                bar_lat: bar.lat,
                bar_lon: bar.lon,
                rank: rank + 1,
                stop_id: &stop.id,
                stop_code: &stop.code,
                stop_name: &stop.name,
                stop_lat: stop.lat,
                stop_lon: stop.lon,
                straight_line_m: straight.round() as u64,
                walk_distance_m: w.map(|(d, _)| d.round() as u64),
                walk_duration_s: w.map(|(_, t)| t.round() as u64),
            })?;
        }

        if (bi + 1) % 25 == 0 || bi + 1 == bars.len() {
            eprintln!("  processed {}/{} bars", bi + 1, bars.len());
        }
    }

    wtr.flush()?;
    eprintln!("Wrote {}", args.out.display());
    Ok(())
}
