use std::collections::BTreeMap;

const fn fnv1a_64(data: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325; const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET; let mut i = 0;
    while i < data.len() { hash ^= data[i] as u64; hash = hash.wrapping_mul(PRIME); i += 1; } hash
}

fn srgb_eotf(v: f64) -> f64 { if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) } }
fn bt709_eotf(v: f64) -> f64 { if v < 0.081 { v / 4.5 } else { ((v + 0.099) / 1.099).powf(1.0 / 0.45) } }
fn bt2020_12_eotf(v: f64) -> f64 { if v < 0.0181*4.5 { v / 4.5 } else { ((v + 0.0993) / 1.0993).powf(1.0/0.45) } }

enum Trc { Para(Vec<f64>), Lut(Vec<u16>), Gamma(f64) }
fn eval_p(p:&[f64],x:f64)->f64{match p.len(){1=>x.powf(p[0]),3=>{let(g,a,b)=(p[0],p[1],p[2]);if x>=-b/a{(a*x+b).powf(g)}else{0.0}}
    5=>{let(g,a,b,c,d)=(p[0],p[1],p[2],p[3],p[4]);if x>=d{(a*x+b).powf(g)}else{c*x}}
    7=>{let(g,a,b,c,d,e,f)=(p[0],p[1],p[2],p[3],p[4],p[5],p[6]);if x>=d{(a*x+b).powf(g)+e}else{c*x+f}}_=>x}}
fn eval(t:&Trc,x:f64)->f64{match t{Trc::Para(p)=>eval_p(p,x),Trc::Gamma(g)=>x.powf(*g),
    Trc::Lut(l)=>{let p=x*(l.len()-1) as f64;let i=p.floor() as usize;let f=p-i as f64;
        if i>=l.len()-1{l[l.len()-1] as f64/65535.0}else{let a=l[i] as f64/65535.0;let b=l[i+1] as f64/65535.0;a+f*(b-a)}}}}
fn parse_trc(d:&[u8],o:usize)->Option<Trc>{if o+12>d.len(){return None;}match &d[o..o+4]{
    b"para"=>{let ft=u16::from_be_bytes([d[o+8],d[o+9]]);let n=match ft{0=>1,1=>3,2=>4,3=>5,4=>7,_=>return None};
        let mut p=Vec::new();for i in 0..n{let q=o+12+i*4;if q+4>d.len(){return None;}p.push(i32::from_be_bytes([d[q],d[q+1],d[q+2],d[q+3]]) as f64/65536.0);}Some(Trc::Para(p))}
    b"curv"=>{let c=u32::from_be_bytes([d[o+8],d[o+9],d[o+10],d[o+11]]) as usize;
        if c==0{Some(Trc::Gamma(1.0))}else if c==1{Some(Trc::Gamma(u16::from_be_bytes([d[o+12],d[o+13]]) as f64/256.0))}
        else{let mut l=Vec::with_capacity(c);for i in 0..c{let q=o+12+i*2;if q+2>d.len(){break;}l.push(u16::from_be_bytes([d[q],d[q+1]]));}Some(Trc::Lut(l))}} _=>None}}
fn find_tag(d:&[u8],s:&[u8;4])->Option<usize>{if d.len()<132{return None;}let tc=u32::from_be_bytes([d[128],d[129],d[130],d[131]]) as usize;
    for i in 0..tc{let b=132+i*12;if b+12>d.len(){break;}if &d[b..b+4]==s{return Some(u32::from_be_bytes([d[b+4],d[b+5],d[b+6],d[b+7]]) as usize);}}None}

struct KP{name:&'static str,cp:u8,rx:f64,ry:f64,gx:f64,gy:f64,bx:f64,by:f64}
const KNOWN_P:&[KP]=&[
    KP{name:"sRGB/BT.709",cp:1,rx:0.4361,ry:0.2225,gx:0.3851,gy:0.7169,bx:0.1431,by:0.0606},
    KP{name:"Display P3",cp:12,rx:0.5151,ry:0.2412,gx:0.2919,gy:0.6922,bx:0.1572,by:0.0666},
    KP{name:"BT.2020",cp:9,rx:0.6734,ry:0.2790,gx:0.1656,gy:0.6753,bx:0.1251,by:0.0456},
];

fn max_u16_err(trc:&Trc,eotf:fn(f64)->f64)->(u32,u32){let mut mx=0u32;let mut gt1=0u32;
    for i in 0..=65535u16{let x=i as f64/65535.0;let a=(eval(trc,x)*65535.0).round() as i64;let b=(eotf(x)*65535.0).round() as i64;
        let d=(a-b).unsigned_abs() as u32;if d>1{gt1+=1;}if d>mx{mx=d;}}(mx,gt1)}

fn main() {
    let dirs: &[&str] = &[
        "/home/lilith/work/zen/zenjpeg/internal/jpegli-cpp/testdata/external/Compact-ICC-Profiles/profiles",
        "/home/lilith/work/zen/zenjpeg/internal/jpegli-cpp/third_party/skcms/profiles/misc",
        "/home/lilith/work/zen/zenjpeg/internal/jpegli-cpp/third_party/skcms/profiles/mobile",
        "/home/lilith/work/zen/zenjpeg/internal/jpegli-cpp/third_party/skcms/profiles/color.org",
        "/home/lilith/work/zen/zenjpeg/internal/jpegli-cpp/third_party/skcms/profiles",
        "/usr/share/color/icc/colord",
        "/usr/share/color/icc/ghostscript",
        "/usr/share/nip2/data",
    ];
    
    let mut results: BTreeMap<String, String> = BTreeMap::new();
    
    for dir in dirs {
        let entries = match std::fs::read_dir(dir) { Ok(e)=>e, Err(_)=>continue };
        for entry in entries {
            let entry = match entry { Ok(e)=>e, Err(_)=>continue };
            let path = entry.path();
            let ext = path.extension().and_then(|e|e.to_str()).unwrap_or("");
            if ext != "icc" && ext != "icm" { continue; }
            let data = match std::fs::read(&path) { Ok(d)=>d, Err(_)=>continue };
            if data.len() < 132 { continue; }
            // Must be RGB 'mntr' profile
            if data.len() >= 20 && &data[12..16] != b"mntr" { continue; }
            if data.len() >= 24 && &data[16..20] != b"RGB " { continue; }
            
            let fname = path.file_name().unwrap().to_string_lossy().to_string();
            let hash = fnv1a_64(&data);
            
            // Identify primaries
            let (r,g,b) = {
                let mut rx=0.0;let mut ry=0.0;let mut gx=0.0;let mut gy=0.0;let mut bx=0.0;let mut by=0.0;
                let tc=u32::from_be_bytes([data[128],data[129],data[130],data[131]]) as usize;
                for i in 0..tc{let base=132+i*12;if base+12>data.len(){break;}
                    let sig=&data[base..base+4];let off=u32::from_be_bytes([data[base+4],data[base+5],data[base+6],data[base+7]]) as usize;
                    if off+20>data.len(){continue;}
                    let rd=|o:usize|(i32::from_be_bytes([data[o+8],data[o+9],data[o+10],data[o+11]]) as f64/65536.0,
                        i32::from_be_bytes([data[o+12],data[o+13],data[o+14],data[o+15]]) as f64/65536.0);
                    match sig{b"rXYZ"=>{let v=rd(off);rx=v.0;ry=v.1;}b"gXYZ"=>{let v=rd(off);gx=v.0;gy=v.1;}b"bXYZ"=>{let v=rd(off);bx=v.0;by=v.1;}_=>{}}}
                ((rx,ry),(gx,gy),(bx,by))
            };
            
            let primaries = {
                let mut found = None;
                for k in KNOWN_P {
                    const T:f64=0.003;
                    if (r.0-k.rx).abs()<T&&(r.1-k.ry).abs()<T&&(g.0-k.gx).abs()<T&&(g.1-k.gy).abs()<T&&(b.0-k.bx).abs()<T&&(b.1-k.by).abs()<T {
                        found = Some((k.cp, k.name)); break;
                    }
                }
                found
            };
            
            let Some((cp, cp_name)) = primaries else { continue; }; // skip non-standard primaries
            
            // Parse TRC
            let trc_off = match find_tag(&data, b"rTRC") { Some(o)=>o, None=>continue };
            let trc = match parse_trc(&data, trc_off) { Some(t)=>t, None=>continue };
            
            // Test against all reference curves
            let (srgb_max, srgb_gt1) = max_u16_err(&trc, srgb_eotf);
            let (bt709_max, bt709_gt1) = max_u16_err(&trc, bt709_eotf);
            let (bt2020_max, _) = max_u16_err(&trc, bt2020_12_eotf);
            
            // Pick best match
            let (best_tc, best_name, best_max, best_gt1) = if srgb_max <= bt709_max && srgb_max <= bt2020_max {
                (13u8, "sRGB", srgb_max, srgb_gt1)
            } else if bt2020_max <= bt709_max {
                (1u8, "BT.2020-12", bt2020_max, 0u32) // recount
            } else {
                (1u8, "BT.709", bt709_max, bt709_gt1)
            };
            
            let key = format!("{:016x}", hash);
            if results.contains_key(&key) { continue; }
            
            let verdict = if best_max <= 3 { "INCLUDE" } else if best_max <= 13 { "INTENT" } else { "SKIP" };
            results.insert(key, format!(
                "{verdict:7} 0x{hash:016x} CP={cp:2}({cp_name:12}) TC={best_tc:2}({best_name:10}) max±{best_max:3} >1={best_gt1:6} {fname} ({} B)",
                data.len()
            ));
        }
    }
    
    println!("{:<7} {:<20} {:<16} {:<14} {:<12} {:<8} {}", "VERDICT", "HASH", "PRIMARIES", "BEST_TRC", "MAX_U16", ">1_CNT", "FILE");
    println!("{}", "-".repeat(120));
    for (_, line) in &results {
        println!("{line}");
    }
    println!("\nTotal RGB monitor profiles with known primaries: {}", results.len());
    println!("INCLUDE (±3): {}", results.values().filter(|v|v.starts_with("INCLUDE")).count());
    println!("INTENT  (±4-13, encode intent was this space): {}", results.values().filter(|v|v.starts_with("INTENT")).count());
    println!("SKIP    (>13): {}", results.values().filter(|v|v.starts_with("SKIP")).count());
}
