#[cfg(target_os = "windows")]
const USAGE: &str = "
  Usage:
    miniredis_mem.exe FILE get KEY
    miniredis_mem.exe FILE delete KEY
    miniredis_mem.exe FILE insert KEY VALUE
    miniredis_mem.exe FILE update KEY VALUE
";

#[cfg(not(target_os = "windows"))]
const USAGE: &str = "
  Usage:
    miniredis_mem FILE get KEY
    miniredis_mem FILE delete KEY
    miniredis_mem FILE insert KEY VALUE
    miniredis_mem FILE update KEY VALUE
";

fn main() {}
