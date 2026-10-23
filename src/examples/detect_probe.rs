//! 直接调 generic::detect 看它为什么认不出来
use dlp_core::pack::generic;
fn main() {
    for path in std::env::args().skip(1) {
        println!("{path}: detect = {:?}", generic::detect(std::path::Path::new(&path)).map(|l| (l.format, l.mc, l.loader, l.prefix)));
    }
}
