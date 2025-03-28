# SE-Clavier Back-end
## Overview

The back-end http service runs on `0.0.0.0:80`

## Building

Install the build-time dependence `racket`.
```sh
sudo apt install racket 
```

Clone this repo and init submodule.
```sh
git clone https://github.com/se-clavier/back-end.git
cd back-end
git submodule update --init --recursive
```

Build with `cargo build`.
