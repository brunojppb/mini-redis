# Mini-redis

Toy implementation of a key-value store with data corruption safety baked in using the [Bitcask disk format.](https://en.wikipedia.org/wiki/Bitcask)

## Building

You must have [Cargo](https://doc.rust-lang.org/book/ch01-03-hello-cargo.html) installed. Then, just run:

```shell
cargo run --bin miniredis_mem data.miniredisdb insert my_key my_value
```

This should create a file named `data.miniredisdb` containing the structured log data. Here is how each entry is structured in the database file:

```log
                             12 bytes header                                     variable length contents
                                  |                                                          |
 |--------------------------------|---------------------------------|  |---------------------|----------------------
 |      Checksum               key length             value length  |  |         key                   value       |
 +------+------+------+ +------+------+------+ +------+------+------+  +--------------------+ +--------------------+
 |      |      |      | |      |      |      | |      |      |      |  |                    | |                    |
 +------+------+------+ +------+------+------+ +------+------+------+  +--------------------+ +--------------------+
 |                    | |                    | |                    |  |                    | |                    |
 |----------|---------| |----------|---------| |----------|---------|  |----------|---------| |----------|---------|
 |                      |                      |                    |  |                    | |                    |
           u32                    u32                    u32              [u8, key length]      [u8, value length]
```

## Usage

This application supports `get`, `delete`, `insert` and `update` operations. Here is how to use a built binary:

```log
Usage:
    miniredis_mem FILE get KEY
    miniredis_mem FILE delete KEY
    miniredis_mem FILE insert KEY VALUE
    miniredis_mem FILE update KEY VALUE
```
