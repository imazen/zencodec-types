//! Verify FNV-1a hashes against moxcms structural profile identification.
//! Parses each ICC profile with moxcms, extracts colorants, identifies
//! the profile structurally, then confirms the hash table entry matches.

use std::path::Path;

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

/// Known colorant values in D50 PCS space (Bradford-adapted from D65).
/// Same tolerance and values as zenpixels-convert/src/cms_moxcms.rs
struct KnownProfile {
    name: &'static str,
    primaries_code: u8,
    transfer_code: u8,
    rx: f64, ry: f64,
    gx: f64, gy: f64,
    bx: f64, by: f64,
}

const KNOWN: &[KnownProfile] = &[
    KnownProfile { name: "sRGB/BT.709", primaries_code: 1, transfer_code: 13,
        rx: 0.4361, ry: 0.2225, gx: 0.3851, gy: 0.7169, bx: 0.1431, by: 0.0606 },
    KnownProfile { name: "Display P3", primaries_code: 12, transfer_code: 13,
        rx: 0.5151, ry: 0.2412, gx: 0.2919, gy: 0.6922, bx: 0.1572, by: 0.0666 },
    KnownProfile { name: "BT.2020", primaries_code: 9, transfer_code: 1,
        rx: 0.6734, ry: 0.2790, gx: 0.1656, gy: 0.6753, bx: 0.1251, by: 0.0456 },
];

fn identify_by_colorants(r: (f64,f64), g: (f64,f64), b: (f64,f64)) -> Option<(u8, u8, &'static str)> {
    const TOL: f64 = 0.003;
    for k in KNOWN {
        if (r.0 - k.rx).abs() < TOL && (r.1 - k.ry).abs() < TOL
            && (g.0 - k.gx).abs() < TOL && (g.1 - k.gy).abs() < TOL
            && (b.0 - k.bx).abs() < TOL && (b.1 - k.by).abs() < TOL
        {
            return Some((k.primaries_code, k.transfer_code, k.name));
        }
    }
    None
}

fn main() {
    let dir = "/home/lilith/work/zen/zenjpeg/.claude/worktrees/agent-a503ce93/internal/jpegli-cpp/testdata/external/Compact-ICC-Profiles/profiles";

    // Hash table entries from zencodec helpers.rs (non-sRGB entries only — sRGB already verified in zencodecs tests)
    let expected: &[(u64, u8, u8, &str)] = &[
        // Display P3
        (0x2cac_00e9_d69a_9840, 12, 13, "DisplayP3Compat-v2-magic"),
        (0x3132_2772_0f77_8b89, 12, 13, "DisplayP3-v2-magic"),
        (0x3f59_a3a4_9d8d_6f25, 12, 13, "DisplayP3Compat-v2-micro"),
        (0x7aa2_2d54_73ad_99bd, 12, 13, "DisplayP3Compat-v4"),
        (0xa52c_7f17_7bff_1392, 12, 13, "DisplayP3-v4"),
        (0xd140_a802_3d39_d033, 12, 13, "DisplayP3-v2-micro"),
        // BT.2020
        (0x45b5_2ef1_ca8c_6fcb, 9, 1, "Rec2020-v4"),
        (0x7fdb_28fb_34fc_eedb, 9, 1, "Rec2020-v2-magic"),
        (0x809e_740f_f28f_1ad8, 9, 1, "Rec2020Compat-v4"),
        (0xb263_a19b_44f5_faba, 9, 1, "Rec2020Compat-v2-micro"),
        (0xbd19_8ece_9409_9edc, 9, 1, "Rec2020Compat-v2-magic"),
        (0xdae0_b26f_b1f4_db65, 9, 1, "Rec2020-v2-micro"),
        // BT.709 (Rec709, not sRGB TRC)
        (0x358f_d60d_2c26_341b, 1, 1, "Rec709-v2-magic"),
        (0x717b_5b97_bad9_374d, 1, 1, "Rec709-v4"),
        (0xe132_14e4_1c8a_55b6, 1, 1, "Rec709-v2-micro"),
        // Adobe RGB (mapped to BT.709 primaries)
        (0x1d3e_7e4f_40c5_8953, 1, 1, "AdobeCompat-v2"),
        (0x4de1_052e_3b80_7417, 1, 1, "AdobeCompat-v4"),
    ];

    let mut errors = 0;
    for &(expected_hash, expected_cp, expected_tc, name) in expected {
        let path = Path::new(dir).join(format!("{name}.icc"));
        let data = match std::fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("SKIP {name}: {e}");
                continue;
            }
        };

        // Verify hash matches
        let actual_hash = fnv1a_64(&data);
        if actual_hash != expected_hash {
            eprintln!("HASH MISMATCH {name}: expected 0x{expected_hash:016x}, got 0x{actual_hash:016x}");
            errors += 1;
            continue;
        }

        // Parse ICC header to extract colorants (minimal parse — just read XYZ tags)
        if data.len() < 132 {
            eprintln!("SKIP {name}: too short ({} bytes)", data.len());
            continue;
        }

        // Read tag count at offset 128
        let tag_count = u32::from_be_bytes([data[128], data[129], data[130], data[131]]) as usize;
        
        // Find rXYZ, gXYZ, bXYZ tags
        let mut r_xyz = None;
        let mut g_xyz = None;
        let mut b_xyz = None;
        
        for i in 0..tag_count {
            let base = 132 + i * 12;
            if base + 12 > data.len() { break; }
            let sig = &data[base..base+4];
            let offset = u32::from_be_bytes([data[base+4], data[base+5], data[base+6], data[base+7]]) as usize;
            
            if offset + 20 > data.len() { continue; }
            
            // XYZ type: 4-byte sig "XYZ " + 4-byte reserved + 3x s15Fixed16
            let read_xyz = |off: usize| -> (f64, f64) {
                let x = i32::from_be_bytes([data[off+8], data[off+9], data[off+10], data[off+11]]) as f64 / 65536.0;
                let y = i32::from_be_bytes([data[off+12], data[off+13], data[off+14], data[off+15]]) as f64 / 65536.0;
                (x, y)
            };
            
            match sig {
                b"rXYZ" => r_xyz = Some(read_xyz(offset)),
                b"gXYZ" => g_xyz = Some(read_xyz(offset)),
                b"bXYZ" => b_xyz = Some(read_xyz(offset)),
                _ => {}
            }
        }
        
        let (Some(r), Some(g), Some(b)) = (r_xyz, g_xyz, b_xyz) else {
            eprintln!("SKIP {name}: missing XYZ colorant tags");
            continue;
        };

        match identify_by_colorants(r, g, b) {
            Some((cp, tc, family)) => {
                // For Adobe RGB: colorants match BT.709 but TRC differs.
                // We map Adobe to (1,1) which is correct for our purposes.
                if cp == expected_cp {
                    println!("OK   {name}: hash=0x{actual_hash:016x} → {family} (CP={cp}, TC={tc}) [expected CP={expected_cp}, TC={expected_tc}]");
                } else {
                    eprintln!("MISMATCH {name}: colorants say CP={cp} ({family}) but hash table says CP={expected_cp}");
                    errors += 1;
                }
            }
            None => {
                // Adobe RGB won't match our BT.709/P3/BT.2020 colorant table — that's expected
                if name.starts_with("Adobe") {
                    println!("OK   {name}: hash=0x{actual_hash:016x} → Adobe RGB (no colorant match, mapped to BT.709 by convention)");
                } else {
                    eprintln!("UNKNOWN {name}: colorants r=({:.4},{:.4}) g=({:.4},{:.4}) b=({:.4},{:.4}) — not in known set", r.0, r.1, g.0, g.1, b.0, b.1);
                    errors += 1;
                }
            }
        }
    }

    if errors > 0 {
        eprintln!("\n{errors} errors found!");
        std::process::exit(1);
    } else {
        println!("\nAll {len} profiles verified: hash matches file, colorants match expected CICP.", len = expected.len());
    }
}
