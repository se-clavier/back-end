# SE-Clavier Back-end
## Overview
[![codecov](https://codecov.io/gh/se-clavier/back-end/graph/badge.svg?token=G7S0ZC1XPS)](https://codecov.io/gh/se-clavier/back-end)

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
