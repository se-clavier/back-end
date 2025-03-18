# SE-Clavier Back-end
## Overview

The back-end http service runs on `0.0.0.0:3000`

## Building

Firstly clone this repo and init submodule.
```sh
git clone https://github.com/se-clavier/back-end.git
cd back-end
git submodule update --init --recursive
```

Then prepare the submodule.
```sh
make -C api prepare #install racket
make -C api src/lib.rs
```

When preparation is done, build with `cargo build`.
