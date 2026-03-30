fn bt2020_12bit_eotf(v: f64) -> f64 {
    if v < 0.0181 * 4.5 { v / 4.5 } else { ((v + 0.0993) / 1.0993).powf(1.0 / 0.45) }
}
enum Trc { Parametric(Vec<f64>), Lut(Vec<u16>), Gamma(f64) }
fn eval_parametric(params: &[f64], x: f64) -> f64 {
    match params.len() { 1=>x.powf(params[0]),
        5=>{let(g,a,b,c,d)=(params[0],params[1],params[2],params[3],params[4]);if x>=d{(a*x+b).powf(g)}else{c*x}}
        7=>{let(g,a,b,c,d,e,f)=(params[0],params[1],params[2],params[3],params[4],params[5],params[6]);if x>=d{(a*x+b).powf(g)+e}else{c*x+f}} _=>x } }
fn eval_trc(trc:&Trc,x:f64)->f64{match trc{Trc::Parametric(p)=>eval_parametric(p,x),Trc::Gamma(g)=>x.powf(*g),
    Trc::Lut(l)=>{let p=x*(l.len()-1) as f64;let i=p.floor() as usize;let f=p-i as f64;if i>=l.len()-1{l[l.len()-1] as f64/65535.0}else{let a=l[i] as f64/65535.0;let b=l[i+1] as f64/65535.0;a+f*(b-a)}}}}
fn parse_trc(data:&[u8],offset:usize)->Option<Trc>{if offset+12>data.len(){return None;}
    match &data[offset..offset+4]{b"para"=>{let ft=u16::from_be_bytes([data[offset+8],data[offset+9]]);let n=match ft{0=>1,1=>3,2=>4,3=>5,4=>7,_=>return None};
        let mut p=Vec::new();for i in 0..n{let o=offset+12+i*4;if o+4>data.len(){return None;}p.push(i32::from_be_bytes([data[o],data[o+1],data[o+2],data[o+3]]) as f64/65536.0);}Some(Trc::Parametric(p))}
    b"curv"=>{let c=u32::from_be_bytes([data[offset+8],data[offset+9],data[offset+10],data[offset+11]]) as usize;
        if c==0{Some(Trc::Gamma(1.0))}else if c==1{Some(Trc::Gamma(u16::from_be_bytes([data[offset+12],data[offset+13]]) as f64/256.0))}
        else{let mut l=Vec::with_capacity(c);for i in 0..c{let o=offset+12+i*2;if o+2>data.len(){break;}l.push(u16::from_be_bytes([data[o],data[o+1]]));}Some(Trc::Lut(l))}} _=>None}}
fn find_tag(data:&[u8],sig:&[u8;4])->Option<usize>{let tc=u32::from_be_bytes([data[128],data[129],data[130],data[131]]) as usize;
    for i in 0..tc{let b=132+i*12;if b+12>data.len(){break;}if &data[b..b+4]==sig{return Some(u32::from_be_bytes([data[b+4],data[b+5],data[b+6],data[b+7]]) as usize);}}None}

fn main() {
    let base = "/home/lilith/work/zen/zenjpeg/internal/jpegli-cpp/testdata/external/Compact-ICC-Profiles/profiles";
    let files = ["Rec2020-v4","Rec2020-v2-magic","Rec2020-v2-micro","Rec2020Compat-v4","Rec2020Compat-v2-magic","Rec2020Compat-v2-micro"];
    println!("{:<30} {:>8} {:>8}", "Profile", "max_u16", ">1_cnt");
    for name in files {
        let data = std::fs::read(format!("{base}/{name}.icc")).unwrap();
        let off = find_tag(&data, b"rTRC").unwrap();
        let trc = parse_trc(&data, off).unwrap();
        let mut max_diff=0u32;let mut gt1=0u32;
        for i in 0..=65535u16{let x=i as f64/65535.0;let a=(eval_trc(&trc,x)*65535.0).round() as i64;let b=(bt2020_12bit_eotf(x)*65535.0).round() as i64;
            let d=(a-b).unsigned_abs() as u32;if d>1{gt1+=1;}if d>max_diff{max_diff=d;}}
        println!("{name:<30} {max_diff:>8} {gt1:>8}");
    }
}
