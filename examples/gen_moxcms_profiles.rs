use moxcms::ColorProfile;

fn fnv1a_64(data: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

fn normalize_and_hash(data: &[u8]) -> u64 {
    let mut d = data.to_vec();
    for (s, e) in [(4usize,8usize), (24,36), (40,44), (48,56), (80,84), (84,100)] {
        for i in s..e.min(d.len()) { d[i] = 0; }
    }
    fnv1a_64(&d)
}

fn main() {
    let profiles: Vec<(&str, ColorProfile)> = vec![
        ("sRGB", ColorProfile::new_srgb()),
        ("Display P3", ColorProfile::new_display_p3()),
        ("Display P3 PQ", ColorProfile::new_display_p3_pq()),
        ("BT.2020", ColorProfile::new_bt2020()),
        ("BT.2020 PQ", ColorProfile::new_bt2020_pq()),
        ("BT.2020 HLG", ColorProfile::new_bt2020_hlg()),
    ];
    
    println!("{:<20} {:>6} {:<18} {:<18}", "NAME", "SIZE", "RAW_HASH", "NORM_HASH");
    for (name, profile) in &profiles {
        match profile.encode() {
            Ok(icc) => {
                let raw = fnv1a_64(&icc);
                let norm = normalize_and_hash(&icc);
                println!("{:<20} {:>6} 0x{:016x} 0x{:016x}", name, icc.len(), raw, norm);
                let fname = format!("/tmp/icc-extraction/moxcms_{}.icc", 
                    name.to_lowercase().replace(' ', "_"));
                std::fs::write(&fname, &icc).unwrap();
            }
            Err(e) => println!("{:<20} FAILED: {:?}", name, e),
        }
    }
}
