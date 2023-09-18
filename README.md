![speed maki image](assets/fast_maki.jpg)
# MakiMaxiSearch

Welcome to MakiMaxiSearch, a fast search command for Linux designed to handle large volumes of data with speed and efficiency.
("MakiMaxiSearch" is pronounced "Mah-kee Mah-xee Search" in English. In Japanese, it is pronounced "まきまきサーチ")

## Description

MakiMaxiSearch is a powerful search tool that is capable of searching through 100GB files in just one second, using only a 100MB index. It is specifically designed for engineers who need high-speed, efficient document search capabilities, especially for system logs. MakiMaxiSearch is not only incredibly fast, but also easy to use and integrate into your existing workflow.

**This respository is on very early stage...**

## Features

- **Fast**: Search 100GB files in just one second.
- **Efficient**: Uses only a 100MB index.
- **Easy to Use**: Simple command-line interface.
- **Versatile**: Ideal for searching system logs and other large documents.

## Installation

To install MakiMaxiSearch, follow these steps:

1. Clone the repository:
```
git clone https://github.com/yourusername/MakiMaxiSearch.git
```
2. Navigate to the project folder:
```
cd maxi-search
```
3. Compile the project:
```
cargo build --release
```

4. Install
```
sudo mv target/release/mmsearch /usr/local/bin
```

## Usage

To use MakiMaxiSearch, create an index of target file and query

1. create index
Run mmsearch like below:
```
mmsearch --file TARGET_FILE --create
```
This creates default index file "index.dat" at current directory.

2. search
At same directory, run it
```
mmsearch --file TARGET_FILE --query "STRING"
```
Then mmsearch write corresnponding chunk to stdout. This is very probablistic because mmsearch only checks if triplets of given STRING exits. So, usually you should run with grep or other tools like.
```
mmsearch --file TARGET_FILE --query "STRING" | grep "STRING"
```
3. Here is full options of mmsearch

```
Usage: mmsearch [OPTIONS] --file <FILE>

Options:
  -f, --file <FILE>    target file
  -i, --index <INDEX>  index file [default: index.dat]
  -c, --chunk <CHUNK>  chunk size should be 2^N [default: 4M] [possible values: 4M, 8M, 16M]
  -l, --log <LOG>      Set the logging level. Options: [error, warn, info, debug, trace] [default: info] [possible values: debug, info, warn]
  -q, --query <QUERY>  query string
  -C, --create         create index
  -h, --help           Print help
  -V, --version        Print version
```

## TODO
- LZ4 compression
- string match function

## Contributing

Contributions are welcome! Please read our [contributing guidelines](CONTRIBUTING.md) for more information.

## License

MakiMaxiSearch is licensed under the [MIT License](LICENSE).

---

Feel free to modify the text to better suit your project's needs or to add any additional sections that you think are necessary.
