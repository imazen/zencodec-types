use std::path::Path;

fn bt709_eotf(v: f64) -> f64 {
    if v < 0.081 { v / 4.5 } else { ((v + 0.099) / 1.099).powf(1.0 / 0.45) }
}
// BT.2020 12-bit uses slightly different constants
fn bt2020_12bit_eotf(v: f64) -> f64 {
    if v < 0.0181 * 4.5 { v / 4.5 } else { ((v + 0.0993) / 1.0993).powf(1.0 / 0.45) }
}
fn gamma24(v: f64) -> f64 { v.powf(2.4) }

enum Trc { Parametric(Vec<f64>), Lut(Vec<u16>), Gamma(f64) }

fn eval_parametric(params: &[f64], x: f64) -> f64 {
    match params.len() {
        1 => x.powf(params[0]),
        5 => { let (g,a,b,c,d) = (params[0],params[1],params[2],params[3],params[4]);
            if x >= d { (a*x+b).powf(g) } else { c*x } }
        7 => { let (g,a,b,c,d,e,f) = (params[0],params[1],params[2],params[3],params[4],params[5],params[6]);
            if x >= d { (a*x+b).powf(g)+e } else { c*x+f } }
        _ => x,
    }
}
fn eval_trc(trc: &Trc, x: f64) -> f64 {
    match trc { Trc::Parametric(p)=>eval_parametric(p,x), Trc::Gamma(g)=>x.powf(*g),
        Trc::Lut(l) => { let p=x*(l.len()-1) as f64; let i=p.floor() as usize; let f=p-i as f64;
            if i>=l.len()-1{l[l.len()-1] as f64/65535.0} else {let a=l[i] as f64/65535.0;let b=l[i+1] as f64/65535.0;a+f*(b-a)} } }
}

fn parse_trc(data: &[u8], offset: usize) -> Option<Trc> {
    if offset+12>data.len(){return None;}
    match &data[offset..offset+4] {
        b"para" => { let ft=u16::from_be_bytes([data[offset+8],data[offset+9]]);
            let n=match ft{0=>1,1=>3,2=>4,3=>5,4=>7,_=>return None};
            let mut p=Vec::new(); for i in 0..n{let o=offset+12+i*4;if o+4>data.len(){return None;}
                p.push(i32::from_be_bytes([data[o],data[o+1],data[o+2],data[o+3]]) as f64/65536.0);}
            Some(Trc::Parametric(p)) }
        b"curv" => { let c=u32::from_be_bytes([data[offset+8],data[offset+9],data[offset+10],data[offset+11]]) as usize;
            if c==0{Some(Trc::Gamma(1.0))} else if c==1{Some(Trc::Gamma(u16::from_be_bytes([data[offset+12],data[offset+13]]) as f64/256.0))}
            else{let mut l=Vec::with_capacity(c);for i in 0..c{let o=offset+12+i*2;if o+2>data.len(){break;}l.push(u16::from_be_bytes([data[o],data[o+1]]));}Some(Trc::Lut(l))} }
        _ => None
    }
}
fn find_tag(data:&[u8],sig:&[u8;4])->Option<usize>{let tc=u32::from_be_bytes([data[128],data[129],data[130],data[131]]) as usize;
    for i in 0..tc{let b=132+i*12;if b+12>data.len(){break;}if &data[b..b+4]==sig{return Some(u32::from_be_bytes([data[b+4],data[b+5],data[b+6],data[b+7]]) as usize);}}None}

fn measure(trc: &Trc, eotf: fn(f64)->f64, name: &str) -> (u32, u32) {
    let mut max_diff=0u32; let mut gt1=0u32;
    for i in 0..=65535u16 { let x=i as f64/65535.0;
        let a=(eval_trc(trc,x)*65535.0).round() as i64;
        let b=(eotf(x)*65535.0).round() as i64;
        let d=(a-b).unsigned_abs() as u32;
        if d>1{gt1+=1;} if d>max_diff{max_diff=d;}
    }
    println!("  vs {name:20}: max_u16={max_diff:3}, >1_cnt={gt1:6}");
    (max_diff, gt1)
}

fn main() {
    let base = "/home/lilith/work/zen/zenjpeg/internal/jpegli-cpp/testdata/external/Compact-ICC-Profiles/profiles";
    let skcms = "/home/lilith/work/zen/zenjpeg/internal/jpegli-cpp/third_party/skcms/profiles/misc";
    
    let files: &[(&str, &str)] = &[
        ("Compact-ICC Rec2020-v4", &format!("{base}/Rec2020-v4.icc")),
        ("Compact-ICC Rec2020Compat-v4", &format!("{base}/Rec2020Compat-v4.icc")),
        ("Compact-ICC Rec2020-g24-v4", &format!("{base}/Rec2020-g24-v4.icc")),
        ("skcms Rec2020_PQ_cicp", &format!("{skcms}/Rec2020_PQ_cicp.icc")),
        ("skcms Rec2020_HLG_cicp", &format!("{skcms}/Rec2020_HLG_cicp.icc")),
    ];
    
    for (label, path) in files {
        let data = match std::fs::read(path) { Ok(d)=>d, Err(e)=>{println!("{label}: {e}"); continue;} };
        println!("\n{label} ({} bytes):", data.len());
        
        // Dump TRC params
        if let Some(off) = find_tag(&data, b"rTRC") {
            if let Some(ref trc) = parse_trc(&data, off) {
                match trc {
                    Trc::Parametric(p) => println!("  TRC: parametric type {} params={:?}", 
                        match p.len(){1=>0,3=>1,4=>2,5=>3,7=>4,_=>99}, p),
                    Trc::Gamma(g) => println!("  TRC: gamma {g:.4}"),
                    Trc::Lut(l) => println!("  TRC: LUT with {} entries", l.len()),
                }
                measure(trc, bt709_eotf, "BT.709");
                measure(trc, bt2020_12bit_eotf, "BT.2020-12bit");
                measure(trc, gamma24, "gamma 2.4");
            }
        }
        
        // Check for CICP tag
        if let Some(off) = find_tag(&data, b"cicp") {
            if off + 12 <= data.len() {
                let cp = data[off+8];
                let tc = data[off+9];
                let mc = data[off+10];
                let fr = data[off+11];
                println!("  CICP: CP={cp} TC={tc} MC={mc} FR={fr}");
            }
        } else {
            println!("  CICP: none");
        }
    }
}
