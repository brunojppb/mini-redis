use lib_miniredis::MiniRedis;

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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let filename = args.get(1).expect(&USAGE);
    let action = args.get(2).expect(&USAGE).as_ref();
    let key = args.get(3).expect(&USAGE).as_ref();
    let maybe_value = args.get(4);

    let path = std::path::Path::new(&filename);
    let mut store =
        MiniRedis::open(path).expect(format!("Could not open the given file: {:?}", path).as_str());
    store.load().expect("Could not load data from file.");

    match action {
        "get" => match store.get(key).unwrap() {
            None => eprintln!("key \"{:?}\" not found", key),
            Some(value) => println!("{:?}", value),
        },

        "delete" => store.delete(key).unwrap(),

        "insert" => {
            let value = maybe_value.expect(&USAGE).as_ref();
            store.insert(key, value).unwrap();
        }

        "update" => {
            let value = maybe_value.expect(&USAGE).as_ref();
            store.update(key, value).unwrap();
        }

        _ => eprint!("{}", &USAGE),
    }
}
