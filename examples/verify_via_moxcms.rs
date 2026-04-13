//! Three-way ICC profile TRC verification: mega_test EOTF vs moxcms evaluation vs canonical.
//!
//! For each ICC profile, evaluates the TRC at all 65536 u16 values using:
//!   1. Our mega_test reference EOTF math (f64)
//!   2. moxcms's TRC evaluator (f32, no CICP shortcuts, no fixed-point, no LUT cache)
//!   3. Canonical EOTF definitions (f64 — the ground truth)
//!
//! Reports max u16 error for each path and flags any disagreements.
//!
//! Build & run:
//!   ICC_PROFILES_DIR=/tmp/icc-extraction/all cargo run --release --example verify_via_moxcms

use moxcms::ColorProfile;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn profile_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("ICC_PROFILES_DIR") {
        return PathBuf::from(dir);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    PathBuf::from(home).join(".cache/zencodec-icc")
}

const fn fnv1a_64(data: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    let mut i = 0;
    while i < data.len() {
        hash ^= data[i] as u64;
        hash = hash.wrapping_mul(PRIME);
        i += 1;
    }
    hash
}

// ── Canonical reference EOTFs (f64, ground truth) ────────────────────────

fn srgb_eotf(v: f64) -> f64 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}
fn bt709_eotf(v: f64) -> f64 {
    if v < 0.081 { v / 4.5 } else { ((v + 0.099) / 1.099).powf(1.0 / 0.45) }
}
fn gamma22_eotf(v: f64) -> f64 { v.powf(2.19921875) } // Adobe RGB: 563/256
fn gamma18_eotf(v: f64) -> f64 { v.powf(1.8) }

// ── mega_test TRC parser (our standalone implementation) ─────────────────

enum Trc { Para(Vec<f64>), Lut(Vec<u16>), Gamma(f64) }

fn eval_para(p: &[f64], x: f64) -> f64 {
    match p.len() {
        1 => x.powf(p[0]),
        3 => { let (g, a, b) = (p[0], p[1], p[2]); if x >= -b / a { (a * x + b).powf(g) } else { 0.0 } }
        5 => { let (g, a, b, c, d) = (p[0], p[1], p[2], p[3], p[4]); if x >= d { (a * x + b).powf(g) } else { c * x } }
        7 => { let (g, a, b, c, d, e, f) = (p[0], p[1], p[2], p[3], p[4], p[5], p[6]); if x >= d { (a * x + b).powf(g) + e } else { c * x + f } }
        _ => x,
    }
}

fn eval_trc(t: &Trc, x: f64) -> f64 {
    match t {
        Trc::Para(p) => eval_para(p, x),
        Trc::Gamma(g) => x.powf(*g),
        Trc::Lut(l) => {
            let p = x * (l.len() - 1) as f64;
            let i = p.floor() as usize;
            let f = p - i as f64;
            if i >= l.len() - 1 { l[l.len() - 1] as f64 / 65535.0 }
            else { let a = l[i] as f64 / 65535.0; let b = l[i + 1] as f64 / 65535.0; a + f * (b - a) }
        }
    }
}

fn parse_trc_from_icc(d: &[u8]) -> Option<Trc> {
    if d.len() < 132 { return None; }
    let tc = u32::from_be_bytes(d[128..132].try_into().ok()?) as usize;
    for i in 0..tc.min(100) {
        let b = 132 + i * 12;
        if b + 12 > d.len() { break; }
        if &d[b..b+4] != b"rTRC" { continue; }
        let o = u32::from_be_bytes(d[b+4..b+8].try_into().ok()?) as usize;
        if o + 12 > d.len() { return None; }
        match &d[o..o+4] {
            b"para" => {
                let ft = u16::from_be_bytes([d[o+8], d[o+9]]);
                let n = match ft { 0 => 1, 1 => 3, 2 => 4, 3 => 5, 4 => 7, _ => return None };
                let mut p = Vec::new();
                for j in 0..n {
                    let q = o + 12 + j * 4;
                    if q + 4 > d.len() { return None; }
                    p.push(i32::from_be_bytes([d[q], d[q+1], d[q+2], d[q+3]]) as f64 / 65536.0);
                }
                return Some(Trc::Para(p));
            }
            b"curv" => {
                let c = u32::from_be_bytes([d[o+8], d[o+9], d[o+10], d[o+11]]) as usize;
                if c == 0 { return Some(Trc::Gamma(1.0)); }
                if c == 1 { return Some(Trc::Gamma(u16::from_be_bytes([d[o+12], d[o+13]]) as f64 / 256.0)); }
                let mut l = Vec::with_capacity(c);
                for j in 0..c { let q = o + 12 + j * 2; if q + 2 > d.len() { break; } l.push(u16::from_be_bytes([d[q], d[q+1]])); }
                return Some(Trc::Lut(l));
            }
            _ => return None,
        }
    }
    None
}

// ── Primaries identification (same as mega_test) ──────────────────────────

struct KP { name: &'static str, cp: u8, rx: f64, ry: f64, gx: f64, gy: f64, bx: f64, by: f64 }
const KNOWN_P: &[KP] = &[
    KP { name: "sRGB/BT.709", cp: 1,   rx: 0.4361, ry: 0.2225, gx: 0.3851, gy: 0.7169, bx: 0.1431, by: 0.0606 },
    KP { name: "Display P3",  cp: 12,  rx: 0.5151, ry: 0.2412, gx: 0.2919, gy: 0.6922, bx: 0.1572, by: 0.0666 },
    KP { name: "BT.2020",     cp: 9,   rx: 0.6734, ry: 0.2790, gx: 0.1656, gy: 0.6753, bx: 0.1251, by: 0.0456 },
    KP { name: "Adobe RGB",   cp: 200, rx: 0.6097, ry: 0.3111, gx: 0.2053, gy: 0.6257, bx: 0.1492, by: 0.0632 },
    KP { name: "ProPhoto",    cp: 201, rx: 0.7977, ry: 0.2880, gx: 0.1352, gy: 0.7119, bx: 0.0313, by: 0.0001 },
];

fn identify_primaries(data: &[u8]) -> Option<(u8, &'static str)> {
    if data.len() < 132 { return None; }
    let tc = u32::from_be_bytes(data[128..132].try_into().ok()?) as usize;
    let (mut r, mut g, mut b) = ((0.0f64, 0.0f64), (0.0f64, 0.0f64), (0.0f64, 0.0f64));
    for i in 0..tc.min(100) {
        let base = 132 + i * 12;
        if base + 12 > data.len() { break; }
        let sig = &data[base..base+4];
        let off = u32::from_be_bytes(data[base+4..base+8].try_into().ok()?) as usize;
        if off + 20 > data.len() { continue; }
        let rd = |o: usize| (
            i32::from_be_bytes(data[o+8..o+12].try_into().unwrap()) as f64 / 65536.0,
            i32::from_be_bytes(data[o+12..o+16].try_into().unwrap()) as f64 / 65536.0,
        );
        match sig { b"rXYZ" => r = rd(off), b"gXYZ" => g = rd(off), b"bXYZ" => b = rd(off), _ => {} }
    }
    for k in KNOWN_P {
        const T: f64 = 0.003;
        if (r.0 - k.rx).abs() < T && (r.1 - k.ry).abs() < T
            && (g.0 - k.gx).abs() < T && (g.1 - k.gy).abs() < T
            && (b.0 - k.bx).abs() < T && (b.1 - k.by).abs() < T
        { return Some((k.cp, k.name)); }
    }
    None
}

// ── Three-way measurement ─────────────────────────────────────────────────

struct ThreeWayResult {
    ref_name: &'static str,
    mega_err: u32,     // mega_test TRC parser vs canonical EOTF
    moxcms_err: u32,   // moxcms TRC evaluator vs canonical EOTF
    divergence: u32,    // mega_test TRC parser vs moxcms TRC evaluator (direct)
}

fn three_way_measure(icc_data: &[u8]) -> Result<Vec<ThreeWayResult>, String> {
    // 1. Parse TRC with our mega_test parser
    let trc = parse_trc_from_icc(icc_data).ok_or("no rTRC in mega_test parser")?;

    // 2. Parse with moxcms, get TRC evaluator (floating point, no shortcuts)
    let profile = ColorProfile::new_from_slice(icc_data).map_err(|e| format!("moxcms parse: {e:?}"))?;
    let red_trc = profile.red_trc.as_ref().ok_or("no red TRC in moxcms")?;
    let evaluator = red_trc.make_linear_evaluator().map_err(|e| format!("evaluator: {e:?}"))?;

    let refs: &[(&str, fn(f64) -> f64)] = &[
        ("sRGB", srgb_eotf),
        ("BT.709", bt709_eotf),
        ("gamma2.2", gamma22_eotf),
        ("gamma1.8", gamma18_eotf),
    ];

    let mut results = Vec::new();

    for &(name, eotf_fn) in refs {
        let mut mega_max: u32 = 0;
        let mut moxcms_max: u32 = 0;
        let mut div_max: u32 = 0;

        for v in 0..=65535u16 {
            let x_f64 = v as f64 / 65535.0;
            let x_f32 = v as f32 / 65535.0;

            // Canonical EOTF (f64 ground truth)
            let canonical = (eotf_fn(x_f64) * 65535.0).round() as i64;

            // mega_test TRC evaluation (f64)
            let mega_val = (eval_trc(&trc, x_f64) * 65535.0).round() as i64;

            // moxcms TRC evaluation (f32, professional, no LUT cache)
            let moxcms_linear = evaluator.evaluate_value(x_f32);
            let moxcms_val = (moxcms_linear as f64 * 65535.0).round() as i64;

            let mega_err = (mega_val - canonical).unsigned_abs() as u32;
            let moxcms_err = (moxcms_val - canonical).unsigned_abs() as u32;
            let div = (mega_val - moxcms_val).unsigned_abs() as u32;

            mega_max = mega_max.max(mega_err);
            moxcms_max = moxcms_max.max(moxcms_err);
            div_max = div_max.max(div);
        }

        results.push(ThreeWayResult {
            ref_name: name,
            mega_err: mega_max,
            moxcms_err: moxcms_max,
            divergence: div_max,
        });
    }

    Ok(results)
}

fn main() {
    let dir = profile_dir();
    if !dir.exists() {
        eprintln!("Profile directory not found: {}", dir.display());
        std::process::exit(1);
    }

    println!(
        "{:<5} {:<18} {:>5} {:>5} {:>5} {:>8}  {:<10} {}",
        "STAT", "HASH", "MEGA", "MOX", "DIV", "BEST_TRC", "PRIMARIES", "FILE"
    );
    println!("{}", "-".repeat(105));

    let mut output: BTreeMap<u64, String> = BTreeMap::new();
    let mut ok = 0u32;
    let mut high = 0u32;
    let mut divergent = 0u32;
    let mut fail = 0u32;

    let entries = std::fs::read_dir(&dir).unwrap();
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();

    for path in &paths {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "icc" && ext != "icm" { continue; }
        let data = match std::fs::read(path) { Ok(d) => d, Err(_) => continue };
        if data.len() < 132 || &data[16..20] != b"RGB " { continue; }

        let fname = path.file_name().unwrap().to_string_lossy();
        let hash = fnv1a_64(&data);
        let Some((_cp, cp_name)) = identify_primaries(&data) else { continue; };

        match three_way_measure(&data) {
            Ok(ref results) => {
                // Pick best TRC by minimum of max(mega, moxcms)
                let best = results.iter()
                    .min_by_key(|r| r.mega_err.max(r.moxcms_err))
                    .unwrap();

                let worst_err = best.mega_err.max(best.moxcms_err);
                let status = if best.divergence > 1 {
                    divergent += 1;
                    "DIV"  // mega_test and moxcms disagree on TRC evaluation
                } else if worst_err <= 56 {
                    ok += 1;
                    "OK"
                } else {
                    high += 1;
                    "HIGH"
                };

                let line = format!(
                    "{:<5} 0x{:016x} {:>5} {:>5} {:>5} {:>8}  {:<10} {}",
                    status, hash, best.mega_err, best.moxcms_err, best.divergence,
                    best.ref_name, cp_name, fname
                );
                output.insert(hash, line);
            }
            Err(e) => {
                fail += 1;
                output.insert(hash, format!(
                    "{:<5} 0x{:016x} {:>5} {:>5} {:>5} {:>8}  {:<10} {} ({})",
                    "FAIL", hash, "-", "-", "-", "-", cp_name, fname, e
                ));
            }
        }
    }

    for (_, line) in &output {
        println!("{line}");
    }

    println!("\nTotal RGB with known primaries: {}", ok + high + divergent + fail);
    println!("OK (max ±56, div ≤1):  {ok}");
    println!("HIGH (max >56):        {high}");
    println!("DIV (mega≠moxcms >1):  {divergent}");
    println!("PARSE FAIL:            {fail}");
}
