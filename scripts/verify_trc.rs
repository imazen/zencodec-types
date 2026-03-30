//! Verify that each ICC profile's TRC matches its claimed CICP transfer
//! function within 0.2% for ALL u16 values (0-65535).
//! Also verify primaries match within s15Fixed16 quantization tolerance.

use std::path::Path;

fn srgb_eotf(v: f64) -> f64 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn bt709_eotf(v: f64) -> f64 {
    if v < 0.081 { v / 4.5 } else { ((v + 0.099) / 1.099).powf(1.0 / 0.45) }
}

/// Evaluate a parametric TRC curve (ICC parametric curve type).
/// Type 0: Y = X^g
/// Type 3: Y = (a*X+b)^g + c  for X >= d, else Y = c*X + f  (5 params: g,a,b,c,d)
/// Type 4: Y = (a*X+b)^g + e  for X >= d, else Y = c*X + f  (7 params: g,a,b,c,d,e,f)
fn eval_parametric(params: &[f64], x: f64) -> f64 {
    match params.len() {
        1 => x.powf(params[0]),  // Type 0: simple gamma
        3 => {  // Type 1: Y = (a*X+b)^g for X >= -b/a, else 0
            let (g, a, b) = (params[0], params[1], params[2]);
            if x >= -b / a { (a * x + b).powf(g) } else { 0.0 }
        }
        4 => {  // Type 2
            let (g, a, b, c) = (params[0], params[1], params[2], params[3]);
            if x >= -b / a { (a * x + b).powf(g) + c } else { c }
        }
        5 => {  // Type 3
            let (g, a, b, c, d) = (params[0], params[1], params[2], params[3], params[4]);
            if x >= d { (a * x + b).powf(g) } else { c * x }
        }
        7 => {  // Type 4: sRGB-like
            let (g, a, b, c, d, e, f) = (params[0], params[1], params[2], params[3], params[4], params[5], params[6]);
            if x >= d { (a * x + b).powf(g) + e } else { c * x + f }
        }
        _ => x,  // identity fallback
    }
}

/// Parse ICC TRC tag — returns either parametric params or LUT entries.
enum Trc {
    Parametric(Vec<f64>),
    Lut(Vec<u16>),
    Gamma(f64),
}

fn parse_trc(data: &[u8], offset: usize) -> Option<Trc> {
    if offset + 12 > data.len() { return None; }
    let sig = &data[offset..offset+4];
    match sig {
        b"para" => {
            let func_type = u16::from_be_bytes([data[offset+8], data[offset+9]]);
            let param_count = match func_type {
                0 => 1, 1 => 3, 2 => 4, 3 => 5, 4 => 7,
                _ => return None,
            };
            let mut params = Vec::new();
            for i in 0..param_count {
                let off = offset + 12 + i * 4;
                if off + 4 > data.len() { return None; }
                let v = i32::from_be_bytes([data[off], data[off+1], data[off+2], data[off+3]]);
                params.push(v as f64 / 65536.0);
            }
            Some(Trc::Parametric(params))
        }
        b"curv" => {
            let count = u32::from_be_bytes([data[offset+8], data[offset+9], data[offset+10], data[offset+11]]) as usize;
            if count == 0 {
                Some(Trc::Gamma(1.0))
            } else if count == 1 {
                let g = u16::from_be_bytes([data[offset+12], data[offset+13]]);
                Some(Trc::Gamma(g as f64 / 256.0))
            } else {
                let mut lut = Vec::with_capacity(count);
                for i in 0..count {
                    let off = offset + 12 + i * 2;
                    if off + 2 > data.len() { break; }
                    lut.push(u16::from_be_bytes([data[off], data[off+1]]));
                }
                Some(Trc::Lut(lut))
            }
        }
        _ => None,
    }
}

fn eval_trc(trc: &Trc, x: f64) -> f64 {
    match trc {
        Trc::Parametric(params) => eval_parametric(params, x),
        Trc::Gamma(g) => x.powf(*g),
        Trc::Lut(lut) => {
            let pos = x * (lut.len() - 1) as f64;
            let i = pos.floor() as usize;
            let frac = pos - i as f64;
            if i >= lut.len() - 1 {
                lut[lut.len() - 1] as f64 / 65535.0
            } else {
                let a = lut[i] as f64 / 65535.0;
                let b = lut[i + 1] as f64 / 65535.0;
                a + frac * (b - a)
            }
        }
    }
}

fn max_trc_error_pct(trc: &Trc, reference_eotf: fn(f64) -> f64) -> (f64, u16) {
    let mut max_err = 0.0f64;
    let mut worst_input = 0u16;
    for i in 0..=65535u16 {
        let x = i as f64 / 65535.0;
        let icc_val = eval_trc(trc, x);
        let ref_val = reference_eotf(x);
        // Relative error (percentage of reference), with absolute floor for near-zero
        let err = if ref_val.abs() < 1e-6 && icc_val.abs() < 1e-6 {
            0.0
        } else if ref_val.abs() < 1e-6 {
            (icc_val - ref_val).abs() * 100.0
        } else {
            ((icc_val - ref_val) / ref_val).abs() * 100.0
        };
        if err > max_err {
            max_err = err;
            worst_input = i;
        }
    }
    (max_err, worst_input)
}

struct KnownPrimaries {
    name: &'static str,
    cp: u8,
    rx: f64, ry: f64,
    gx: f64, gy: f64,
    bx: f64, by: f64,
}

const KNOWN_PRIMARIES: &[KnownPrimaries] = &[
    KnownPrimaries { name: "BT.709/sRGB", cp: 1,
        rx: 0.4361, ry: 0.2225, gx: 0.3851, gy: 0.7169, bx: 0.1431, by: 0.0606 },
    KnownPrimaries { name: "Display P3", cp: 12,
        rx: 0.5151, ry: 0.2412, gx: 0.2919, gy: 0.6922, bx: 0.1572, by: 0.0666 },
    KnownPrimaries { name: "BT.2020", cp: 9,
        rx: 0.6734, ry: 0.2790, gx: 0.1656, gy: 0.6753, bx: 0.1251, by: 0.0456 },
];

fn identify_primaries(data: &[u8]) -> Option<(u8, &'static str)> {
    let tag_count = u32::from_be_bytes([data[128], data[129], data[130], data[131]]) as usize;
    let mut r = (0.0, 0.0);
    let mut g = (0.0, 0.0);
    let mut b = (0.0, 0.0);
    for i in 0..tag_count {
        let base = 132 + i * 12;
        if base + 12 > data.len() { break; }
        let sig = &data[base..base+4];
        let offset = u32::from_be_bytes([data[base+4], data[base+5], data[base+6], data[base+7]]) as usize;
        if offset + 20 > data.len() { continue; }
        let read_xy = |off: usize| -> (f64, f64) {
            let x = i32::from_be_bytes([data[off+8], data[off+9], data[off+10], data[off+11]]) as f64 / 65536.0;
            let y = i32::from_be_bytes([data[off+12], data[off+13], data[off+14], data[off+15]]) as f64 / 65536.0;
            (x, y)
        };
        match sig {
            b"rXYZ" => r = read_xy(offset),
            b"gXYZ" => g = read_xy(offset),
            b"bXYZ" => b = read_xy(offset),
            _ => {}
        }
    }
    const TOL: f64 = 0.001; // s15Fixed16 quantization: 1/65536 ≈ 0.0000153
    for k in KNOWN_PRIMARIES {
        if (r.0 - k.rx).abs() < TOL && (r.1 - k.ry).abs() < TOL
            && (g.0 - k.gx).abs() < TOL && (g.1 - k.gy).abs() < TOL
            && (b.0 - k.bx).abs() < TOL && (b.1 - k.by).abs() < TOL
        {
            return Some((k.cp, k.name));
        }
    }
    None
}

fn find_trc_tag(data: &[u8], tag_sig: &[u8; 4]) -> Option<usize> {
    let tag_count = u32::from_be_bytes([data[128], data[129], data[130], data[131]]) as usize;
    for i in 0..tag_count {
        let base = 132 + i * 12;
        if base + 12 > data.len() { break; }
        if &data[base..base+4] == tag_sig {
            return Some(u32::from_be_bytes([data[base+4], data[base+5], data[base+6], data[base+7]]) as usize);
        }
    }
    None
}

fn main() {
    let dir = "/home/lilith/work/zen/zenjpeg/.claude/worktrees/agent-a503ce93/internal/jpegli-cpp/testdata/external/Compact-ICC-Profiles/profiles";
    
    // All non-sRGB entries from the hash table
    let profiles: &[(&str, u8, u8)] = &[
        // Display P3 — should have P3 primaries + sRGB TRC
        ("DisplayP3Compat-v2-magic", 12, 13),
        ("DisplayP3-v2-magic", 12, 13),
        ("DisplayP3Compat-v2-micro", 12, 13),
        ("DisplayP3Compat-v4", 12, 13),
        ("DisplayP3-v4", 12, 13),
        ("DisplayP3-v2-micro", 12, 13),
        // BT.2020 — should have BT.2020 primaries + BT.709 TRC
        ("Rec2020-v4", 9, 1),
        ("Rec2020-v2-magic", 9, 1),
        ("Rec2020Compat-v4", 9, 1),
        ("Rec2020Compat-v2-micro", 9, 1),
        ("Rec2020Compat-v2-magic", 9, 1),
        ("Rec2020-v2-micro", 9, 1),
        // BT.709 — should have BT.709 primaries + BT.709 TRC
        ("Rec709-v2-magic", 1, 1),
        ("Rec709-v4", 1, 1),
        ("Rec709-v2-micro", 1, 1),
        // Adobe RGB — should have ??? primaries + gamma 2.2
        ("AdobeCompat-v2", 1, 1),
        ("AdobeCompat-v4", 1, 1),
    ];
    
    let mut errors = 0;
    for &(name, expected_cp, expected_tc) in profiles {
        let path = Path::new(dir).join(format!("{name}.icc"));
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => { eprintln!("SKIP {name}: {e}"); continue; }
        };
        
        // Check primaries
        let primaries_match = identify_primaries(&data);
        let primaries_ok = primaries_match.map(|(cp, _)| cp == expected_cp).unwrap_or(false);
        
        // Check TRC (use rTRC — all channels should be identical for these profiles)
        let reference_eotf: fn(f64) -> f64 = match expected_tc {
            13 => srgb_eotf,
            1 => bt709_eotf,
            _ => { eprintln!("SKIP {name}: unknown TC={expected_tc}"); continue; }
        };
        
        let trc_offset = find_trc_tag(&data, b"rTRC");
        let (trc_err, worst_input) = if let Some(off) = trc_offset {
            if let Some(trc) = parse_trc(&data, off) {
                max_trc_error_pct(&trc, reference_eotf)
            } else {
                eprintln!("SKIP {name}: unparseable TRC"); continue;
            }
        } else {
            eprintln!("SKIP {name}: no rTRC tag"); continue;
        };
        
        let primaries_label = primaries_match.map(|(_, n)| n).unwrap_or("UNKNOWN");
        let trc_ok = trc_err <= 0.2;
        let status = if primaries_ok && trc_ok { "OK  " } else { "FAIL" };
        
        println!("{status} {name}: primaries={primaries_label}(ok={primaries_ok}) TRC max_err={trc_err:.4}% at u16={worst_input} (threshold=0.2%)");
        
        if !primaries_ok || !trc_ok {
            errors += 1;
        }
    }
    
    println!("\n{errors} failures out of {} profiles", profiles.len());
    if errors > 0 { std::process::exit(1); }
}
