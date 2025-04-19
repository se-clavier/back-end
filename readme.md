# SE-Clavier Back-end
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=se-clavier_back-end&metric=coverage)](https://sonarcloud.io/summary/new_code?id=se-clavier_back-end) [![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=se-clavier_back-end&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=se-clavier_back-end) [![Maintainability Rating](https://sonarcloud.io/api/project_badges/measure?project=se-clavier_back-end&metric=sqale_rating)](https://sonarcloud.io/summary/new_code?id=se-clavier_back-end)
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
