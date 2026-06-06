use sikuwa_core::{Codename, Version, VERSION};

pub fn run() -> i32 {
    println!("sikuwa {}", VERSION);
    println!("codename: {}", Codename::NAME);
    println!("engine:   {}", Codename::ENGINE);
    if let Some(v) = Version::parse(VERSION) {
        println!("semver:   {}.{}.{}", v.major, v.minor, v.patch);
    }
    0
}
