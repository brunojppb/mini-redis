use lib_miniredis::{ByteStr, ByteString, MiniRedis};
use std::collections::HashMap;

#[cfg(target_os = "windows")]
const USAGE: &str = "
Usage:
  miniredis.exe FILE get KEY
  miniredis.exe FILE delete KEY
  miniredis.exe FILE insert KEY VALUE
  miniredis.exe FILE update KEY VALUE
";

#[cfg(not(target_os = "windows"))]
const USAGE: &str = "
Usage:
  miniredis FILE get KEY
  miniredis FILE delete KEY
  miniredis FILE insert KEY VALUE
  miniredis FILE update KEY VALUE
";

//
fn store_index_on_disk(store: &mut MiniRedis, index_key: &ByteStr) {
    store.index.remove(index_key);
    let index_as_bytes = bincode::serialize(&store.index).unwrap();
    store.index = HashMap::new();
    store.insert(index_key, &index_as_bytes).unwrap();
}

fn main() {
    const INDEX_KEY: &ByteStr = b"+index";

    let args: Vec<String> = std::env::args().collect();
    let filename = args.get(1).expect(&USAGE);
    let action = args.get(2).expect(&USAGE).as_ref();
    let key = args.get(3).expect(&USAGE).as_ref();
    let maybe_value = args.get(4);

    let path = std::path::Path::new(&filename);
    let mut store = MiniRedis::open(path).expect("Could not open the given file.");

    // @TODO: There is still a problem here:
    // When we call `load`, it will rebuild the index from scratch, which defeats
    // the purpose of having the already persisted index on disk
    store
        .load()
        .expect("Could not load data from the given file.");

    match action {
        "get" => {
            let index_as_bytes = store.get(&INDEX_KEY).unwrap().unwrap();
            let decoded_index = bincode::deserialize(&index_as_bytes);
            let index: HashMap<ByteString, u64> = decoded_index.unwrap();

            match index.get(key) {
                None => eprintln!("Key not found. Key={:?}", key),
                Some(&i) => {
                    let pair = store.get_at(i).unwrap();
                    // Values can potentially be just bytes, with no encoding attached.
                    // so we use the Debug trait to print the value.
                    println!("{:?}", pair.value);
                }
            }
        }

        "delete" => store.delete(key).unwrap(),

        "insert" => {
            let value = maybe_value.expect(&USAGE).as_ref();
            store.insert(key, value).unwrap();
            store_index_on_disk(&mut store, &INDEX_KEY);
        }

        "update" => {
            let value = maybe_value.expect(&USAGE).as_ref();
            store.update(key, value).unwrap();
            store_index_on_disk(&mut store, &INDEX_KEY);
        }

        _ => eprintln!("{}", &USAGE),
    };
}
